use std::collections::HashMap;

use engine::registry::Registry;
use itertools::Itertools;
use parser_sdl::{GraphqlDirective, NeonDirective, OpenApiDirective, ParseResult};

#[tokio::main]
pub async fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    assert!(args.len() == 3);
    let schema_path = &args[1];
    let registry_path = &args[2];

    let schema = tokio::fs::read_to_string(schema_path).await.unwrap();
    let environment = std::env::vars().collect::<HashMap<String, String>>();
    let registry = parse_schema(&schema, &environment).await.unwrap();
    tokio::fs::write(registry_path, serde_json::to_vec(&registry).unwrap())
        .await
        .unwrap();
}

/// Transform the input schema into a Registry
pub async fn parse_schema(
    schema: &str,
    environment: &HashMap<String, String>,
) -> anyhow::Result<engine::Registry> {
    let connector_parsers = ConnectorParsers {
        http_client: reqwest::Client::new(),
    };

    let ParseResult {
        mut registry,
        global_cache_rules,
        ..
    } = parser_sdl::parse(schema, environment, true, &connector_parsers).await?;

    // apply global caching rules
    global_cache_rules
        .apply(&mut registry)
        .map_err(|e| anyhow::anyhow!(e.into_iter().join("\n")))?;

    Ok(registry)
}

struct ConnectorParsers {
    http_client: reqwest::Client,
}

#[async_trait::async_trait]
impl parser_sdl::ConnectorParsers for ConnectorParsers {
    async fn fetch_and_parse_openapi(
        &self,
        directive: OpenApiDirective,
    ) -> Result<Registry, Vec<String>> {
        let mut request = self.http_client.get(&directive.schema_url);

        for (name, value) in directive.introspection_headers() {
            request = request.header(name, value);
        }

        let response = request.send().await.map_err(|e| vec![e.to_string()])?;

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|header_value| header_value.to_str().ok())
            .map(ToOwned::to_owned);

        let spec = response.text().await.map_err(|e| vec![e.to_string()])?;

        let format = parser_openapi::Format::guess(content_type.as_deref(), &directive.schema_url);

        let mut registry = Registry::new();

        parser_openapi::parse_spec(spec, format, directive.into(), &mut registry).map_err(
            |errors| {
                errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
            },
        )?;

        Ok(registry)
    }

    async fn fetch_and_parse_graphql(
        &self,
        directive: GraphqlDirective,
    ) -> Result<Registry, Vec<String>> {
        parser_graphql::parse_schema(
            self.http_client.clone(),
            &directive.name,
            directive.namespace,
            &directive.url,
            directive.headers(),
            directive.introspection_headers(),
        )
        .await
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
        })
    }

    async fn fetch_and_parse_neon(
        &self,
        _directive: &NeonDirective,
    ) -> Result<Registry, Vec<String>> {
        Err(vec!["Not implemented".to_string()])
    }
}
