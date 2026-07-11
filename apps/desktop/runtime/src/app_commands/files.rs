use super::*;

pub fn file_watch(
    service: &RuntimeService,
    project_path: String,
) -> Result<crate::files::FileWatchRegistration, String> {
    service.file_watch(project_path)
}
pub fn file_unwatch(service: &RuntimeService, project_path: String) -> Result<(), String> {
    service.file_unwatch(project_path)
}
pub fn file_list_children(
    request: crate::files::FileChildrenRequest,
) -> Result<Vec<crate::files::FileEntry>, String> {
    crate::files::file_list_children(request)
}
pub fn file_read(
    request: crate::files::FilePathRequest,
) -> Result<crate::files::FileReadResult, String> {
    crate::files::file_read(request)
}
pub fn file_write(
    request: crate::files::FileWriteRequest,
) -> Result<crate::files::FileReadResult, String> {
    crate::files::file_write(request)
}
pub fn file_create_file(
    request: crate::files::FileCreateRequest,
) -> Result<crate::files::FileEntry, String> {
    crate::files::file_create_file(request)
}
pub fn file_create_dir(
    request: crate::files::FileCreateRequest,
) -> Result<crate::files::FileEntry, String> {
    crate::files::file_create_dir(request)
}
pub fn file_rename(
    request: crate::files::FileRenameRequest,
) -> Result<crate::files::FileEntry, String> {
    crate::files::file_rename(request)
}
pub fn file_delete(request: crate::files::FilePathRequest) -> Result<(), String> {
    crate::files::file_delete(request)
}
pub fn file_copy(
    request: crate::files::FileCopyRequest,
) -> Result<crate::files::FileEntry, String> {
    crate::files::file_copy(request)
}
pub fn file_import_external(
    request: crate::files::FileExternalCopyRequest,
) -> Result<Vec<crate::files::FileEntry>, String> {
    crate::files::file_import_external(request)
}
pub fn file_reveal(request: crate::files::FilePathRequest) -> Result<(), String> {
    crate::files::file_reveal(request)
}
pub fn file_open(request: crate::files::FilePathRequest) -> Result<(), String> {
    crate::files::file_open(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn file_commands_delegate_to_runtime_files_layer() {
        let support_dir =
            std::env::temp_dir().join(format!("wecode-app-command-files-{}", Uuid::new_v4()));
        let project_dir = support_dir.join("project");
        let external_dir = support_dir.join("external");
        std::fs::create_dir_all(&project_dir).expect("project dir");
        std::fs::create_dir_all(&external_dir).expect("external dir");
        let service = RuntimeService::new(support_dir.clone());

        let registration =
            file_watch(&service, project_dir.display().to_string()).expect("watch project");
        assert_eq!(
            std::path::PathBuf::from(&registration.project_path)
                .canonicalize()
                .expect("canonical registration path"),
            project_dir.canonicalize().expect("canonical project path")
        );

        let dir = file_create_dir(crate::files::FileCreateRequest {
            root_path: project_dir.display().to_string(),
            parent_path: None,
            name: "src".to_string(),
        })
        .expect("create dir");
        assert_eq!(dir.relative_path, "src");

        let file = file_create_file(crate::files::FileCreateRequest {
            root_path: project_dir.display().to_string(),
            parent_path: Some("src".to_string()),
            name: "main.rs".to_string(),
        })
        .expect("create file");
        assert_eq!(file.relative_path, "src/main.rs");

        let written = file_write(crate::files::FileWriteRequest {
            root_path: project_dir.display().to_string(),
            path: "src/main.rs".to_string(),
            content: "fn main() {}\n".to_string(),
        })
        .expect("write file");
        assert_eq!(written.content, "fn main() {}\n");

        let read = file_read(crate::files::FilePathRequest {
            root_path: project_dir.display().to_string(),
            path: "src/main.rs".to_string(),
        })
        .expect("read file");
        assert_eq!(read.name, "main.rs");

        let copied = file_copy(crate::files::FileCopyRequest {
            root_path: project_dir.display().to_string(),
            source_path: "src/main.rs".to_string(),
            target_directory_path: None,
        })
        .expect("copy file");
        assert!(copied.relative_path.starts_with("src/main copy "));
        assert!(project_dir.join(&copied.relative_path).exists());

        let renamed = file_rename(crate::files::FileRenameRequest {
            root_path: project_dir.display().to_string(),
            path: "src/main.rs".to_string(),
            new_name: "lib.rs".to_string(),
        })
        .expect("rename file");
        assert_eq!(renamed.relative_path, "src/lib.rs");

        let external_file = external_dir.join("note.txt");
        std::fs::write(&external_file, "note").expect("write external file");
        let imported = file_import_external(crate::files::FileExternalCopyRequest {
            root_path: project_dir.display().to_string(),
            source_paths: vec![external_file.display().to_string()],
            target_directory_path: Some("src".to_string()),
        })
        .expect("import external file");
        assert_eq!(imported[0].relative_path, "src/note.txt");

        let children = file_list_children(crate::files::FileChildrenRequest {
            root_path: project_dir.display().to_string(),
            directory_path: Some("src".to_string()),
        })
        .expect("list children");
        assert!(children.iter().any(|entry| entry.name == "lib.rs"));
        assert!(children.iter().any(|entry| entry.name == "note.txt"));

        file_delete(crate::files::FilePathRequest {
            root_path: project_dir.display().to_string(),
            path: "src/lib.rs".to_string(),
        })
        .expect("delete file");
        assert!(!project_dir.join("src/lib.rs").exists());

        file_unwatch(&service, project_dir.display().to_string()).expect("unwatch project");

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
