//! topology : [TopologyEndpoint] × [DirectedConnection] ⇀ Topology   (typed endpoints, acyclic, canonical order).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::endpoints::{BoundaryAccount, FiniteCompartment};
use crate::identity::CompartmentId;

/// A structurally classified endpoint in a directed topology.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopologyEndpoint {
    /// A finite compartment endpoint.
    Finite(FiniteCompartment),
    /// A boundary account endpoint.
    Boundary(BoundaryAccount),
}

impl TopologyEndpoint {
    /// Returns the endpoint's compartment identity.
    #[must_use]
    pub fn id(&self) -> &CompartmentId {
        match self {
            Self::Finite(compartment) => compartment.id(),
            Self::Boundary(account) => account.id(),
        }
    }
}

/// A directed connection between two declared topology endpoints.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DirectedConnection {
    source: CompartmentId,
    target: CompartmentId,
}

impl DirectedConnection {
    /// Constructs a directed connection from `source` to `target`.
    #[must_use]
    pub fn new(source: CompartmentId, target: CompartmentId) -> Self {
        Self { source, target }
    }

    /// Returns the source endpoint identity.
    #[must_use]
    pub fn source(&self) -> &CompartmentId {
        &self.source
    }

    /// Returns the target endpoint identity.
    #[must_use]
    pub fn target(&self) -> &CompartmentId {
        &self.target
    }
}

/// Identifies which connection endpoint is undeclared.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionEndpointRole {
    /// The connection source.
    Source,
    /// The connection target.
    Target,
}

impl Display for ConnectionEndpointRole {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source => formatter.write_str("source"),
            Self::Target => formatter.write_str("target"),
        }
    }
}

/// Reports why a directed topology could not be constructed.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum TopologyError {
    /// Returned when multiple endpoints have the same compartment identity.
    #[error("duplicate topology endpoint identity `{endpoint}`")]
    DuplicateEndpoint { endpoint: CompartmentId },
    /// Returned when the same directed connection occurs more than once.
    #[error("duplicate directed connection `{connection_source}` -> `{connection_target}`")]
    DuplicateConnection {
        connection_source: CompartmentId,
        connection_target: CompartmentId,
    },
    /// Returned when a connection endpoint is not declared in the topology.
    #[error("directed connection {role} endpoint `{endpoint}` is not declared in the topology")]
    UnknownConnectionEndpoint {
        role: ConnectionEndpointRole,
        endpoint: CompartmentId,
    },
    /// Returned when directed cycles block a complete topological traversal.
    #[error(
        "topology contains a directed cycle; blocked endpoint identities: {blocked_endpoint_ids:?}"
    )]
    Cycle {
        blocked_endpoint_ids: Vec<CompartmentId>,
    },
}

/// A validated directed topology with canonical endpoint and connection storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Topology {
    endpoints: BTreeMap<CompartmentId, TopologyEndpoint>,
    connections: Vec<DirectedConnection>,
    topological_order: Vec<CompartmentId>,
}

impl Topology {
    /// Constructs a validated, canonically ordered directed topology.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::DuplicateEndpoint`] for a repeated endpoint identity,
    /// [`TopologyError::DuplicateConnection`] for a repeated directed pair,
    /// [`TopologyError::UnknownConnectionEndpoint`] for an undeclared source or target, and
    /// [`TopologyError::Cycle`] when the directed graph is cyclic.
    pub fn new(
        endpoints: impl IntoIterator<Item = TopologyEndpoint>,
        connections: impl IntoIterator<Item = DirectedConnection>,
    ) -> Result<Self, TopologyError> {
        let mut endpoint_map = BTreeMap::new();
        for endpoint in endpoints {
            let endpoint_id = endpoint.id().clone();
            if endpoint_map.insert(endpoint_id.clone(), endpoint).is_some() {
                return Err(TopologyError::DuplicateEndpoint {
                    endpoint: endpoint_id,
                });
            }
        }

        let mut connection_set = BTreeSet::new();
        for connection in connections {
            if !connection_set.insert(connection.clone()) {
                return Err(TopologyError::DuplicateConnection {
                    connection_source: connection.source,
                    connection_target: connection.target,
                });
            }
        }

        for connection in &connection_set {
            if !endpoint_map.contains_key(connection.source()) {
                return Err(TopologyError::UnknownConnectionEndpoint {
                    role: ConnectionEndpointRole::Source,
                    endpoint: connection.source().clone(),
                });
            }
            if !endpoint_map.contains_key(connection.target()) {
                return Err(TopologyError::UnknownConnectionEndpoint {
                    role: ConnectionEndpointRole::Target,
                    endpoint: connection.target().clone(),
                });
            }
        }

        let mut indegrees = endpoint_map
            .keys()
            .cloned()
            .map(|endpoint_id| (endpoint_id, 0_usize))
            .collect::<BTreeMap<_, _>>();
        let mut adjacency = endpoint_map
            .keys()
            .cloned()
            .map(|endpoint_id| (endpoint_id, BTreeSet::new()))
            .collect::<BTreeMap<_, _>>();

        for connection in &connection_set {
            let Some(targets) = adjacency.get_mut(connection.source()) else {
                unreachable!("validated source must have an adjacency entry");
            };
            targets.insert(connection.target().clone());
            let Some(indegree) = indegrees.get_mut(connection.target()) else {
                unreachable!("validated target must have an indegree entry");
            };
            *indegree += 1;
        }

        let mut zero_indegree = indegrees
            .iter()
            .filter(|(_, indegree)| **indegree == 0)
            .map(|(endpoint_id, _)| endpoint_id.clone())
            .collect::<BTreeSet<_>>();
        let mut topological_order = Vec::with_capacity(endpoint_map.len());

        while let Some(endpoint_id) = zero_indegree.pop_first() {
            topological_order.push(endpoint_id.clone());
            let Some(adjacent_targets) = adjacency.get(&endpoint_id) else {
                unreachable!("declared endpoint must have an adjacency entry");
            };
            let targets = adjacent_targets.iter().cloned().collect::<Vec<_>>();
            for target in targets {
                let Some(indegree) = indegrees.get_mut(&target) else {
                    unreachable!("validated target must have an indegree entry");
                };
                *indegree -= 1;
                if *indegree == 0 {
                    zero_indegree.insert(target);
                }
            }
        }

        if topological_order.len() != endpoint_map.len() {
            let blocked_endpoint_ids = indegrees
                .into_iter()
                .filter(|(_, indegree)| *indegree != 0)
                .map(|(endpoint_id, _)| endpoint_id)
                .collect();
            return Err(TopologyError::Cycle {
                blocked_endpoint_ids,
            });
        }

        Ok(Self {
            endpoints: endpoint_map,
            connections: connection_set.into_iter().collect(),
            topological_order,
        })
    }

    /// Returns the endpoint with `id`, when declared.
    #[must_use]
    pub fn endpoint(&self, id: &CompartmentId) -> Option<&TopologyEndpoint> {
        self.endpoints.get(id)
    }

    /// Iterates over endpoints in ascending identity order.
    #[must_use]
    pub fn endpoints(
        &self,
    ) -> impl ExactSizeIterator<Item = &TopologyEndpoint> + DoubleEndedIterator {
        self.endpoints.values()
    }

    /// Returns directed connections in canonical source-target order.
    #[must_use]
    pub fn connections(&self) -> &[DirectedConnection] {
        &self.connections
    }

    /// Returns endpoint identities in stable topological order.
    #[must_use]
    pub fn topological_order(&self) -> &[CompartmentId] {
        &self.topological_order
    }

    /// Returns the number of declared endpoints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.endpoints.len()
    }

    /// Returns whether the topology has no declared endpoints.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty()
    }
}
