mod html;
mod package;
mod types;

#[cfg(test)]
mod tests;

use super::{PetCustomPet, PetCustomPetInstallPreview, PetCustomPetInstallRequest};
use html::{resolve_custom_pet_install_from_html, validate_petdex_url};
use package::install_custom_pet_package;
use std::path::PathBuf;
use url::Url;

pub(super) async fn resolve_custom_pet_install(
    request: PetCustomPetInstallRequest,
) -> Result<PetCustomPetInstallPreview, String> {
    let raw_url = request.page_url.trim();
    let page_url =
        Url::parse(raw_url).map_err(|_| "Please enter a Petdex pet page URL.".to_string())?;
    validate_petdex_url(&page_url)?;
    let html = reqwest::get(page_url.clone())
        .await
        .map_err(|_| "Failed to load the Petdex page.".to_string())?
        .error_for_status()
        .map_err(|_| "Failed to load the Petdex page.".to_string())?
        .text()
        .await
        .map_err(|_| "Unable to read the Petdex page.".to_string())?;
    resolve_custom_pet_install_from_html(request, &html, &page_url)
}

pub(super) async fn install_custom_pet(
    support_dir: PathBuf,
    request: PetCustomPetInstallRequest,
) -> Result<PetCustomPet, String> {
    let preview = resolve_custom_pet_install(request).await?;
    let zip_url = Url::parse(&preview.zip_url)
        .map_err(|_| "The Petdex package URL is invalid.".to_string())?;
    let bytes = reqwest::get(zip_url)
        .await
        .map_err(|_| "Failed to download the pet package.".to_string())?
        .error_for_status()
        .map_err(|_| "Failed to download the pet package.".to_string())?
        .bytes()
        .await
        .map_err(|_| "Failed to download the pet package.".to_string())?;
    install_custom_pet_package(&support_dir, preview, &bytes)
}
