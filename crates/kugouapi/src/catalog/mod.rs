//! The endpoint catalog: 271 endpoints covering every module of the reference
//! implementation.
//!
//! - [`crate::catalog::domains`] — regular gateway endpoints (static metadata +
//!   typed query projections)
//! - [`crate::catalog::special`] — special protocol families (login, encrypted
//!   POST, cloudlist, comments, listen-together, composite flows)
//!
//! Endpoints are invoked through [`crate::client::KuGouClient::call`] or the
//! typed domain APIs on [`crate::client::KuGouClient`].

pub mod ctx;
pub mod def;
pub mod domains;
pub mod special;

use std::collections::HashMap;
use std::sync::LazyLock;

pub use ctx::{GatewayRequest, RawRequest, SignControl, call_ctx};
pub use def::{
    Built, EndpointSpec, InvokeFn, NoParams, RegisteredEndpoint, Salt, handler_endpoint,
    simple_endpoint, spec, unwrap_envelope,
};

/// The global endpoint registry.
pub static REGISTRY: LazyLock<Registry> = LazyLock::new(|| Registry::new(all_endpoints()));

/// Registry of all endpoints, keyed by catalog id.
pub struct Registry {
    map: HashMap<&'static str, RegisteredEndpoint>,
    ids: Vec<&'static str>,
}

impl Registry {
    /// Build from a list of endpoints.
    #[must_use]
    pub fn new(endpoints: Vec<RegisteredEndpoint>) -> Self {
        let mut map = HashMap::with_capacity(endpoints.len());
        let mut ids = Vec::with_capacity(endpoints.len());
        for endpoint in endpoints {
            ids.push(endpoint.spec.id);
            map.insert(endpoint.spec.id, endpoint);
        }
        ids.sort_unstable();
        Self { map, ids }
    }

    /// Look up an endpoint by catalog id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&RegisteredEndpoint> {
        self.map.get(id)
    }

    /// All endpoint ids, sorted.
    #[must_use]
    pub fn ids(&self) -> &[&'static str] {
        &self.ids
    }

    /// Endpoint count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Total endpoint count covered by this crate (all 271 reference modules).
pub const ENDPOINT_COUNT: usize = 271;

/// All endpoint ids, sorted.
pub fn endpoint_ids() -> &'static [&'static str] {
    REGISTRY.ids()
}

/// Look up an endpoint by catalog id.
pub fn endpoint(id: &str) -> Option<&'static RegisteredEndpoint> {
    REGISTRY.get(id)
}

/// The full endpoint list (regular domains first, then special families).
fn all_endpoints() -> Vec<RegisteredEndpoint> {
    let mut all = Vec::new();
    all.extend(domains::endpoints());
    all.extend(special::endpoints());
    all
}
