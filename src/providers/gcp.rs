use super::NuObjectStore;
use crate::cache::{Cache, ObjectStoreCacheKey};
use nu_plugin::EngineInterface;
use nu_protocol::{ShellError, Spanned};
use object_store::gcp::GoogleCloudStorageBuilder;
use std::sync::Arc;
use url::Url;

pub async fn build_object_store(
    engine: &EngineInterface,
    cache: &Cache,
    url: &Spanned<Url>,
) -> Result<NuObjectStore, ShellError> {
    let bucket = parse_bucket(&url.item).ok_or_else(|| ShellError::GenericError {
        error: format!(
            "Could not determine Google Cloud Storage bucket name from url {}",
            url.item
        ),
        msg: "".into(),
        span: Some(url.span),
        help: None,
        inner: vec![],
    })?;

    let cache_key = ObjectStoreCacheKey::GoogleCloudStorage {
        bucket: bucket.clone(),
    };

    if let Some(object_store) = cache.get_store(&cache_key).await {
        Ok(object_store)
    } else {
        let gcs = GoogleCloudStorageBuilder::from_env()
            .with_url(url.item.to_string())
            .build()
            .map_err(|e| ShellError::GenericError {
                error: format!("Could not create Google Cloud Storage client: {e}"),
                msg: "".into(),
                span: Some(url.span),
                help: None,
                inner: vec![],
            })?;

        let object_store = NuObjectStore::GoogleCloudStorage {
            store: Arc::new(gcs),
            bucket,
        };
        cache
            .put_store(engine, cache_key, object_store.clone())
            .await?;
        Ok(object_store)
    }
}

fn parse_bucket(url: &Url) -> Option<String> {
    (url.scheme() == "gs")
        .then(|| url.host_str().map(ToString::to_string))
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::parse_bucket;
    use url::Url;

    #[test]
    fn parses_gs_bucket() {
        let url = Url::parse("gs://my-bucket/path/to/file.csv").expect("valid url");
        assert_eq!(parse_bucket(&url).as_deref(), Some("my-bucket"));
    }

    #[test]
    fn rejects_non_gs_bucket() {
        let url =
            Url::parse("https://storage.googleapis.com/my-bucket/file.csv").expect("valid url");
        assert_eq!(parse_bucket(&url), None);
    }
}
