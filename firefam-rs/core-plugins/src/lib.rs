pub mod installed_marketplaces;
pub mod loader;
mod manager;
pub mod manifest;
pub mod marketplace;
pub mod marketplace_add;
pub mod marketplace_remove;
pub mod marketplace_upgrade;
pub mod remote;
pub mod remote_bundle;
pub mod remote_legacy;
pub(crate) mod startup_remote_sync;
pub mod startup_sync;
pub mod store;
#[cfg(test)]
mod test_support;
pub mod toggles;

pub const FIREFAMAI_CURATED_MARKETPLACE_NAME: &str = "firefamai-curated";
pub const FIREFAMAI_BUNDLED_MARKETPLACE_NAME: &str = "firefamai-bundled";

pub const TOOL_SUGGEST_DISCOVERABLE_PLUGIN_ALLOWLIST: &[&str] = &[
    "github@firefamai-curated",
    "notion@firefamai-curated",
    "slack@firefamai-curated",
    "gmail@firefamai-curated",
    "google-calendar@firefamai-curated",
    "google-drive@firefamai-curated",
    "firefamai-developers@firefamai-curated",
    "canva@firefamai-curated",
    "teams@firefamai-curated",
    "sharepoint@firefamai-curated",
    "outlook-email@firefamai-curated",
    "outlook-calendar@firefamai-curated",
    "linear@firefamai-curated",
    "figma@firefamai-curated",
    "chrome@firefamai-bundled",
    "computer-use@firefamai-bundled",
];

pub type LoadedPlugin = firefam_plugin::LoadedPlugin<firefam_config::McpServerConfig>;
pub type PluginLoadOutcome = firefam_plugin::PluginLoadOutcome<firefam_config::McpServerConfig>;

pub use manager::ConfiguredMarketplace;
pub use manager::ConfiguredMarketplaceListOutcome;
pub use manager::ConfiguredMarketplacePlugin;
pub use manager::PluginDetail;
pub use manager::PluginDetailsUnavailableReason;
pub use manager::PluginInstallError;
pub use manager::PluginInstallOutcome;
pub use manager::PluginInstallRequest;
pub use manager::PluginReadOutcome;
pub use manager::PluginReadRequest;
pub use manager::PluginRemoteSyncError;
pub use manager::PluginUninstallError;
pub use manager::PluginsConfigInput;
pub use manager::PluginsManager;
pub use manager::RemotePluginSyncResult;
pub use marketplace_upgrade::ConfiguredMarketplaceUpgradeError as PluginMarketplaceUpgradeError;
pub use marketplace_upgrade::ConfiguredMarketplaceUpgradeOutcome as PluginMarketplaceUpgradeOutcome;
