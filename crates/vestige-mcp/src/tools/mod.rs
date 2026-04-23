//! MCP Tools

// Active unified tools
pub mod codebase_unified;
pub mod intention_unified;
pub mod memory_unified;
pub mod search_unified;
pub mod smart_ingest;

// v1.2: Temporal query tools
pub mod changelog;
pub mod timeline;

// v1.2: Maintenance tools
pub mod maintenance;

// v1.3: Auto-save and dedup tools
pub mod dedup;
pub mod importance;

// v1.5: Cognitive tools
pub mod dream;
pub mod explore;
pub mod predict;
pub mod restore;

// v1.8: Context Packets
pub mod session_context;

// v1.9: Autonomic tools
pub mod graph;
pub mod health;

// v2.1: Cross-reference (connect the dots)
pub mod cross_reference;

// v2.0.5: Active Forgetting — Anderson 2025 + Davis Rac1
pub mod suppress;

// v2.1: Custom additions
pub mod tags;
pub mod export_claude;
pub mod workspace;

// Internal/backwards-compat
#[allow(dead_code)]
pub mod context;
#[allow(dead_code)]
pub mod feedback;
#[allow(dead_code)]
pub mod memory_states;
#[allow(dead_code)]
pub mod review;
#[allow(dead_code)]
pub mod tagging;
