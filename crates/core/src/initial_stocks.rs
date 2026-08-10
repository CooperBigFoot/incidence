//! initial_stocks : Topology × SubstanceRegistry × [(CompartmentId, SparseSubstanceVector)] ⇀ InitialStocks   (validated, deterministic)

use std::collections::{BTreeMap, BTreeSet};

use crate::identity::{CompartmentId, SubstanceId};
use crate::non_negative_amount::NonNegativeAmount;
use crate::presence::ValueState;
use crate::sparse_substance_vector::SparseSubstanceVector;
use crate::substance_registry::SubstanceRegistry;
use crate::topology::{Topology, TopologyEndpoint};

/// Registry-bound initial stocks for finite compartments in a topology snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct InitialStocks {
    topology: Topology,
    registry: SubstanceRegistry,
    stocks: BTreeMap<CompartmentId, SparseSubstanceVector>,
}

impl InitialStocks {
    /// Binds validated initial stocks to topology and substance-registry snapshots.
    ///
    /// # Errors
    ///
    /// Returns [`InitialStocksError::UnknownCompartment`] for an undeclared identity,
    /// [`InitialStocksError::BoundaryAccountStock`] for a boundary account,
    /// [`InitialStocksError::RegistryMismatch`] for a vector bound to a different registry, or
    /// [`InitialStocksError::DuplicateCompartment`] for a repeated compartment identity.
    pub fn new(
        topology: &Topology,
        registry: &SubstanceRegistry,
        entries: impl IntoIterator<Item = (CompartmentId, SparseSubstanceVector)>,
    ) -> Result<Self, InitialStocksError> {
        let mut seen = BTreeSet::new();
        let mut stocks = BTreeMap::new();

        for (compartment, vector) in entries {
            if !seen.insert(compartment.clone()) {
                return Err(InitialStocksError::DuplicateCompartment { compartment });
            }

            match topology.endpoint(&compartment) {
                None => {
                    return Err(InitialStocksError::UnknownCompartment { compartment });
                }
                Some(TopologyEndpoint::Boundary(_)) => {
                    return Err(InitialStocksError::BoundaryAccountStock {
                        account: compartment,
                    });
                }
                Some(TopologyEndpoint::Finite(_)) => {}
            }

            if vector.registry() != registry {
                return Err(InitialStocksError::RegistryMismatch { compartment });
            }

            stocks.insert(compartment, vector);
        }

        Ok(Self {
            topology: topology.clone(),
            registry: registry.clone(),
            stocks,
        })
    }

    /// Returns the bound topology snapshot.
    #[must_use]
    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Returns the bound substance-registry snapshot.
    #[must_use]
    pub fn registry(&self) -> &SubstanceRegistry {
        &self.registry
    }

    /// Iterates over supplied entries in stable topology order.
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = (&CompartmentId, &SparseSubstanceVector)> {
        self.topology
            .topological_order()
            .iter()
            .filter_map(|compartment| {
                self.stocks
                    .get(compartment)
                    .map(|vector| (compartment, vector))
            })
    }

    /// Returns the initial amount state at one compartment-substance coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`InitialStocksError::UnknownCompartment`] for an undeclared compartment or
    /// [`InitialStocksError::BoundaryAccountStock`] for a boundary account.
    #[must_use]
    pub fn amount(
        &self,
        compartment: &CompartmentId,
        substance: &SubstanceId,
    ) -> Result<ValueState<NonNegativeAmount>, InitialStocksError> {
        match self.topology.endpoint(compartment) {
            None => {
                return Err(InitialStocksError::UnknownCompartment {
                    compartment: compartment.clone(),
                });
            }
            Some(TopologyEndpoint::Boundary(_)) => {
                return Err(InitialStocksError::BoundaryAccountStock {
                    account: compartment.clone(),
                });
            }
            Some(TopologyEndpoint::Finite(_)) => {}
        }

        if let Some(vector) = self.stocks.get(compartment) {
            return Ok(vector.amount(substance));
        }

        if self.registry.contains(substance) {
            Ok(ValueState::Present(NonNegativeAmount::ZERO))
        } else {
            Ok(ValueState::NotModelled)
        }
    }
}

/// Reports why initial stocks could not be bound or queried.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum InitialStocksError {
    /// Fires when construction or lookup names an identity absent from the bound topology.
    #[error("initial stock compartment `{compartment}` is not declared in the topology")]
    UnknownCompartment { compartment: CompartmentId },
    /// Fires when construction or lookup attempts to treat a boundary account as a finite-stock holder.
    #[error("boundary account `{account}` cannot hold initial stock")]
    BoundaryAccountStock { account: CompartmentId },
    /// Fires when a supplied sparse vector's registry snapshot differs from the registry being bound.
    #[error(
        "initial stock for compartment `{compartment}` is bound to a different substance registry"
    )]
    RegistryMismatch { compartment: CompartmentId },
    /// Fires when constructor input repeats a compartment identity.
    #[error("duplicate initial stock entry for compartment `{compartment}`")]
    DuplicateCompartment { compartment: CompartmentId },
}
