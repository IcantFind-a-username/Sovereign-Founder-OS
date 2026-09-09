use crate::http::AssetRoute;

pub(crate) struct EmbeddedAsset {
    pub(crate) content_type: &'static str,
    pub(crate) bytes: &'static [u8],
}

pub(crate) fn asset(route: AssetRoute) -> EmbeddedAsset {
    match route {
        AssetRoute::Index => EmbeddedAsset {
            content_type: "text/html; charset=utf-8",
            bytes: include_bytes!("../assets/index.html"),
        },
        AssetRoute::Styles => EmbeddedAsset {
            content_type: "text/css; charset=utf-8",
            bytes: include_bytes!("../assets/styles.css"),
        },
        AssetRoute::I18n => EmbeddedAsset {
            content_type: "text/javascript; charset=utf-8",
            bytes: include_bytes!("../assets/i18n.js"),
        },
        AssetRoute::App => EmbeddedAsset {
            content_type: "text/javascript; charset=utf-8",
            bytes: include_bytes!("../assets/app.js"),
        },
        AssetRoute::ConsultantUi => EmbeddedAsset {
            content_type: "text/javascript; charset=utf-8",
            bytes: include_bytes!("../assets/consultant-ui.js"),
        },
        AssetRoute::Favicon => EmbeddedAsset {
            content_type: "image/svg+xml",
            bytes: include_bytes!("../assets/favicon.svg"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::asset;
    use crate::http::AssetRoute;

    #[test]
    fn embedded_assets_match_closed_routes_and_mime() {
        let routes = [
            (AssetRoute::Index, "text/html; charset=utf-8", "index.html"),
            (AssetRoute::Styles, "text/css; charset=utf-8", "styles.css"),
            (
                AssetRoute::I18n,
                "text/javascript; charset=utf-8",
                "i18n.js",
            ),
            (AssetRoute::App, "text/javascript; charset=utf-8", "app.js"),
            (
                AssetRoute::ConsultantUi,
                "text/javascript; charset=utf-8",
                "consultant-ui.js",
            ),
            (AssetRoute::Favicon, "image/svg+xml", "favicon.svg"),
        ];
        for (route, mime, filename) in routes {
            let embedded = asset(route);
            assert_eq!(embedded.content_type, mime);
            assert!(!embedded.bytes.is_empty());
            std::str::from_utf8(embedded.bytes).expect("asset is UTF-8");
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(filename);
            let source = std::fs::read(path).expect("asset source exists");
            assert_eq!(embedded.bytes, source, "wrong bytes for {filename}");
        }
    }
}
