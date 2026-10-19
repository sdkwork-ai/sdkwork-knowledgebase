//! Memory adapter for SDKWork Knowledgebase.

mod adapter;
mod app_api;
mod in_process;

pub use adapter::KnowledgebaseMemoryContextProviderAdapter;
pub use app_api::{parse_memory_space_id, MemoryAppApiContextProvider};
pub use in_process::InProcessMemoryContextProvider;
