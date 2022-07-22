use super::resource::{self, Resource};

pub async fn spawn_resource(path: &str) -> (&str, Resource) {
    let resource = resource::load_resource(path).await;

    (path, resource)
}

macro_rules! resolve_all {
    ( $( $p: expr ),* ) => {
        futures::future::join_all(
            vec![
                $(
                    crate::util::resource_util::spawn_resource($p),
                )*
            ]
        ).await
    };
}
