use zed_extension_api::{self as zed, Result};

struct MelbiExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for MelbiExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Option 1: Use bundled binary
        let command = zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: vec![],
            env: Default::default(),
        };
        Ok(command)
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(None)
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(None)
    }
}

impl MelbiExtension {
    fn language_server_binary_path(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path
            && std::fs::metadata(path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(path.clone());
        }

        // Option 1: Look for locally built binary (development)
        if let Some(path) = worktree.which("melbi-lsp") {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        // TODO: Option 2: Download from GitHub releases
        Err("melbi-lsp binary not found".into())
    }
}

zed::register_extension!(MelbiExtension);
