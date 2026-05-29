use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use gpui_component_assets::Assets as ComponentAssets;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "runtime-assets"]
#[include = "rank-icons/**/*.svg"]
struct RuntimeAssets;

pub struct CoduxAssets;

impl AssetSource for CoduxAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match ComponentAssets.load(path) {
            Ok(Some(asset)) => return Ok(Some(asset)),
            Ok(None) => {}
            Err(_) => {}
        }

        RuntimeAssets::get(path)
            .map(|file| Some(file.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut items = ComponentAssets.list(path).unwrap_or_default();
        items.extend(
            RuntimeAssets::iter().filter_map(|item| item.starts_with(path).then(|| item.into())),
        );
        Ok(items)
    }
}
