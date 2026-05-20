//! Test-only helpers exposed for cross-crate integration tests.
//!
//! Production code should not depend on this module.
//! We prefer this to using a crate feature to avoid building multiple
//! permutations of the crate.

use std::path::PathBuf;
use std::sync::Arc;

use firefam_exec_server::EnvironmentManager;
use firefam_login::AuthManager;
use firefam_login::FirefamAuth;
use firefam_model_provider::create_model_provider;
use firefam_model_provider_info::ModelProviderInfo;
use firefam_models_manager::bundled_models_response;
use firefam_models_manager::collaboration_mode_presets;
use firefam_models_manager::manager::SharedModelsManager;
use firefam_models_manager::test_support::construct_model_info_offline_for_tests;
use firefam_models_manager::test_support::get_model_offline_for_tests;
use firefam_protocol::config_types::CollaborationModeMask;
use firefam_protocol::firefamai_models::ModelInfo;
use firefam_protocol::firefamai_models::ModelPreset;
use once_cell::sync::Lazy;

use crate::ThreadManager;
use crate::config::Config;
use crate::thread_manager;
use crate::unified_exec;

static TEST_MODEL_PRESETS: Lazy<Vec<ModelPreset>> = Lazy::new(|| {
    let mut response = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    response.models.sort_by(|a, b| a.priority.cmp(&b.priority));
    let mut presets: Vec<ModelPreset> = response.models.into_iter().map(Into::into).collect();
    ModelPreset::mark_default_by_picker_visibility(&mut presets);
    presets
});

pub fn set_thread_manager_test_mode(enabled: bool) {
    thread_manager::set_thread_manager_test_mode_for_tests(enabled);
}

pub fn set_deterministic_process_ids(enabled: bool) {
    unified_exec::set_deterministic_process_ids_for_tests(enabled);
}

pub fn auth_manager_from_auth(auth: FirefamAuth) -> Arc<AuthManager> {
    AuthManager::from_auth_for_testing(auth)
}

pub fn auth_manager_from_auth_with_home(
    auth: FirefamAuth,
    firefam_home: PathBuf,
) -> Arc<AuthManager> {
    AuthManager::from_auth_for_testing_with_home(auth, firefam_home)
}

pub fn thread_manager_with_models_provider(
    auth: FirefamAuth,
    provider: ModelProviderInfo,
) -> ThreadManager {
    ThreadManager::with_models_provider_for_tests(auth, provider)
}

pub fn thread_manager_with_models_provider_and_home(
    auth: FirefamAuth,
    provider: ModelProviderInfo,
    firefam_home: PathBuf,
    environment_manager: Arc<EnvironmentManager>,
) -> ThreadManager {
    ThreadManager::with_models_provider_and_home_for_tests(
        auth,
        provider,
        firefam_home,
        environment_manager,
    )
}

pub fn thread_manager_with_models_provider_home_and_state(
    auth: FirefamAuth,
    provider: ModelProviderInfo,
    firefam_home: PathBuf,
    environment_manager: Arc<EnvironmentManager>,
    state_db: Option<crate::StateDbHandle>,
) -> ThreadManager {
    ThreadManager::with_models_provider_home_and_state_for_tests(
        auth,
        provider,
        firefam_home,
        environment_manager,
        state_db,
    )
}

pub async fn start_thread_with_user_shell_override(
    thread_manager: &ThreadManager,
    config: Config,
    user_shell_override: crate::shell::Shell,
) -> firefam_protocol::error::Result<crate::NewThread> {
    thread_manager
        .start_thread_with_user_shell_override_for_tests(config, user_shell_override)
        .await
}

pub async fn resume_thread_from_rollout_with_user_shell_override(
    thread_manager: &ThreadManager,
    config: Config,
    rollout_path: PathBuf,
    auth_manager: Arc<AuthManager>,
    user_shell_override: crate::shell::Shell,
) -> firefam_protocol::error::Result<crate::NewThread> {
    thread_manager
        .resume_thread_from_rollout_with_user_shell_override_for_tests(
            config,
            rollout_path,
            auth_manager,
            user_shell_override,
        )
        .await
}

pub fn models_manager_with_provider(
    firefam_home: PathBuf,
    auth_manager: Arc<AuthManager>,
    provider: ModelProviderInfo,
) -> SharedModelsManager {
    let provider = create_model_provider(provider, Some(auth_manager));
    provider.models_manager(firefam_home, /*config_model_catalog*/ None)
}

pub fn get_model_offline(model: Option<&str>) -> String {
    get_model_offline_for_tests(model)
}

pub fn construct_model_info_offline(model: &str, config: &Config) -> ModelInfo {
    construct_model_info_offline_for_tests(model, &config.to_models_manager_config())
}

pub fn all_model_presets() -> &'static Vec<ModelPreset> {
    &TEST_MODEL_PRESETS
}

pub fn builtin_collaboration_mode_presets() -> Vec<CollaborationModeMask> {
    collaboration_mode_presets::builtin_collaboration_mode_presets()
}
