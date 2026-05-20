use std::sync::Arc;
use std::sync::Weak;

use firefam_core::NewThread;
use firefam_core::StartThreadOptions;
use firefam_core::ThreadManager;
use firefam_core::config::Config;
use firefam_extension_api::AgentSpawnFuture;
use firefam_extension_api::AgentSpawner;
use firefam_extension_api::ExtensionRegistry;
use firefam_extension_api::ExtensionRegistryBuilder;
use firefam_protocol::ThreadId;
use firefam_protocol::error::FirefamErr;

pub(crate) fn thread_extensions<S>(guardian_agent_spawner: S) -> Arc<ExtensionRegistry<Config>>
where
    S: AgentSpawner<StartThreadOptions, Spawned = NewThread, Error = FirefamErr> + 'static,
{
    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    firefam_guardian::install(&mut builder, guardian_agent_spawner);
    firefam_memories_extension::install(&mut builder);
    Arc::new(builder.build())
}

pub(crate) fn guardian_agent_spawner(
    thread_manager: Weak<ThreadManager>,
) -> impl AgentSpawner<StartThreadOptions, Spawned = NewThread, Error = FirefamErr> {
    move |forked_from_thread_id: ThreadId,
          options: StartThreadOptions|
          -> AgentSpawnFuture<'static, NewThread, FirefamErr> {
        let thread_manager = thread_manager.clone();
        Box::pin(async move {
            let thread_manager = thread_manager.upgrade().ok_or_else(|| {
                FirefamErr::UnsupportedOperation("thread manager dropped".to_string())
            })?;
            thread_manager
                .spawn_subagent(forked_from_thread_id, options)
                .await
        })
    }
}
