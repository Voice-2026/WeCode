use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListItem {
    pub id: String,
    pub name: String,
    pub path: String,
}

pub fn project_list_payload(
    projects: impl IntoIterator<Item = ProjectListItem>,
    selected_project_id: Option<String>,
) -> Value {
    let projects = projects
        .into_iter()
        .map(|project| {
            json!({
                "id": project.id,
                "name": project.name,
                "path": project.path,
            })
        })
        .collect::<Vec<_>>();
    json!({ "projects": projects, "selectedProjectId": selected_project_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_list_payload_keeps_mobile_shape() {
        let payload = project_list_payload(
            [ProjectListItem {
                id: "project-1".to_string(),
                name: "Codux".to_string(),
                path: "/tmp/codux".to_string(),
            }],
            Some("project-1".to_string()),
        );

        assert_eq!(payload["selectedProjectId"], "project-1");
        assert_eq!(payload["projects"][0]["name"], "Codux");
        assert!(payload["projects"][0].get("badgeText").is_none());
    }
}
