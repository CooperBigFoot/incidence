//! ledger : ModelArtifact × [Record] ⇀ (RunStatus, StockHistory)   (pure, deterministic)
//!
//! Genesis supplies the fold seed, each Transfer contributes one incidence column per
//! substance, and an optional RunCompleted record proves that the declared horizon was reached.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::endpoints::{BoundaryAccount, FiniteCompartment};
use crate::identity::{CompartmentId, SubstanceId};
use crate::initial_stocks::InitialStocks;
use crate::model_artifact::{
    MAX_EXACT_WHOLE_MULTIPLES, ModelArtifact, ModelArtifactArchive, ModelDigest,
};
use crate::non_negative_amount::NonNegativeAmount;
use crate::numerical_semantics::NumericalSemanticsVersion;
use crate::presence::ValueState;
use crate::signed_boundary_balance::SignedBoundaryBalance;
use crate::sparse_substance_vector::SparseSubstanceVector;
use crate::substance_registry::SubstanceRegistry;
use crate::temporal::TimestepIndex;
use crate::topology::{Topology, TopologyEndpoint};

/// The structurally classified endpoint stored by a transfer record.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TransferEndpoint {
    /// A non-negative stock holder.
    Finite(FiniteCompartment),
    /// A signed external counterparty.
    Boundary(BoundaryAccount),
}

impl TransferEndpoint {
    #[must_use]
    pub fn id(&self) -> &CompartmentId {
        match self {
            Self::Finite(value) => value.id(),
            Self::Boundary(value) => value.id(),
        }
    }
    #[must_use]
    pub fn is_finite(&self) -> bool {
        matches!(self, Self::Finite(_))
    }
}
impl From<FiniteCompartment> for TransferEndpoint {
    fn from(value: FiniteCompartment) -> Self {
        Self::Finite(value)
    }
}
impl From<BoundaryAccount> for TransferEndpoint {
    fn from(value: BoundaryAccount) -> Self {
        Self::Boundary(value)
    }
}
impl From<TopologyEndpoint> for TransferEndpoint {
    fn from(value: TopologyEndpoint) -> Self {
        match value {
            TopologyEndpoint::Finite(v) => Self::Finite(v),
            TopologyEndpoint::Boundary(v) => Self::Boundary(v),
        }
    }
}

/// Opaque identity of one run, independent of its model artifact identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunId([u8; 16]);
impl RunId {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// The first record and complete seed of the authoritative stock fold.
#[derive(Clone, Debug, PartialEq)]
pub struct Genesis {
    run_id: RunId,
    model_digest: ModelDigest,
    numerical_semantics: NumericalSemanticsVersion,
    initial_stocks: InitialStocks,
}
impl Genesis {
    /// Creates Genesis from the exact immutable artifact that starts a run.
    #[must_use]
    pub fn for_run(run_id: RunId, artifact: &ModelArtifact) -> Self {
        Self {
            run_id,
            model_digest: artifact.digest(),
            numerical_semantics: artifact.versions().numerical_semantics(),
            initial_stocks: artifact.initial_stocks().clone(),
        }
    }
    /// Constructs a decoded Genesis record. Compatibility is checked during replay.
    #[must_use]
    pub fn new(
        run_id: RunId,
        model_digest: ModelDigest,
        numerical_semantics: NumericalSemanticsVersion,
        initial_stocks: InitialStocks,
    ) -> Self {
        Self {
            run_id,
            model_digest,
            numerical_semantics,
            initial_stocks,
        }
    }
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }
    #[must_use]
    pub fn model_digest(&self) -> ModelDigest {
        self.model_digest
    }
    #[must_use]
    pub fn numerical_semantics_version(&self) -> NumericalSemanticsVersion {
        self.numerical_semantics
    }
    #[must_use]
    pub fn initial_stocks(&self) -> &InitialStocks {
        &self.initial_stocks
    }
}

/// An atomic movement of registered extensive amounts between typed endpoints.
#[derive(Clone, Debug, PartialEq)]
pub struct Transfer {
    timestep: TimestepIndex,
    source: TransferEndpoint,
    target: TransferEndpoint,
    amounts: SparseSubstanceVector,
}
impl Transfer {
    #[must_use]
    pub fn new(
        timestep: TimestepIndex,
        source: impl Into<TransferEndpoint>,
        target: impl Into<TransferEndpoint>,
        amounts: SparseSubstanceVector,
    ) -> Self {
        Self {
            timestep,
            source: source.into(),
            target: target.into(),
            amounts,
        }
    }
    #[must_use]
    pub fn timestep(&self) -> TimestepIndex {
        self.timestep
    }
    #[must_use]
    pub fn source(&self) -> &TransferEndpoint {
        &self.source
    }
    #[must_use]
    pub fn target(&self) -> &TransferEndpoint {
        &self.target
    }
    #[must_use]
    pub fn amounts(&self) -> &SparseSubstanceVector {
        &self.amounts
    }
}

/// SHA-256 identity of Genesis and all Transfers preceding a completion seal.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LogDigest([u8; 32]);
impl LogDigest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    #[must_use]
    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }
}
impl Display for LogDigest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Terminal proof binding the declared final timestep, transfer count, and preceding records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunCompleted {
    final_timestep: TimestepIndex,
    transfer_count: u64,
    log_digest: LogDigest,
}
impl RunCompleted {
    #[must_use]
    pub const fn new(
        final_timestep: TimestepIndex,
        transfer_count: u64,
        log_digest: LogDigest,
    ) -> Self {
        Self {
            final_timestep,
            transfer_count,
            log_digest,
        }
    }
    #[must_use]
    pub const fn final_timestep(&self) -> TimestepIndex {
        self.final_timestep
    }
    #[must_use]
    pub const fn transfer_count(&self) -> u64 {
        self.transfer_count
    }
    #[must_use]
    pub const fn log_digest(&self) -> LogDigest {
        self.log_digest
    }
}

/// One immutable fact in an authoritative log.
#[derive(Clone, Debug, PartialEq)]
pub enum Record {
    Genesis(Genesis),
    Transfer(Transfer),
    RunCompleted(RunCompleted),
}

/// An ordered Genesis-led sequence which is either appendable or terminally sealed.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoritativeLog {
    genesis: Genesis,
    transfers: Vec<Transfer>,
    seal: Option<RunCompleted>,
}
impl AuthoritativeLog {
    #[must_use]
    pub fn new(genesis: Genesis) -> Self {
        Self {
            genesis,
            transfers: Vec::new(),
            seal: None,
        }
    }
    #[must_use]
    pub fn for_run(run_id: RunId, artifact: &ModelArtifact) -> Self {
        Self::new(Genesis::for_run(run_id, artifact))
    }
    #[must_use]
    pub fn genesis(&self) -> &Genesis {
        &self.genesis
    }
    #[must_use]
    pub fn transfers(&self) -> &[Transfer] {
        &self.transfers
    }
    #[must_use]
    pub fn seal_record(&self) -> Option<&RunCompleted> {
        self.seal.as_ref()
    }
    #[must_use]
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }
    #[must_use]
    pub fn is_sealed(&self) -> bool {
        self.seal.is_some()
    }
    /// Appends one transfer, rejecting records after a seal and decreasing timestep order.
    ///
    /// # Errors
    /// Returns [`LogError::RecordAfterSeal`] or [`LogError::TimestepOrder`].
    pub fn append(&mut self, transfer: Transfer) -> Result<(), LogError> {
        if self.seal.is_some() {
            return Err(LogError::RecordAfterSeal);
        }
        if let Some(previous) = self.transfers.last().map(Transfer::timestep)
            && transfer.timestep < previous
        {
            return Err(LogError::TimestepOrder {
                previous,
                next: transfer.timestep,
            });
        }
        self.transfers.push(transfer);
        Ok(())
    }
    /// Seals the sequence at the model-declared final timestep.
    ///
    /// # Errors
    /// Returns an error if already sealed or if the transfer count cannot be represented.
    pub fn seal(&mut self, final_timestep: TimestepIndex) -> Result<&RunCompleted, LogError> {
        if self.seal.is_some() {
            return Err(LogError::AlreadySealed);
        }
        let transfer_count =
            u64::try_from(self.transfers.len()).map_err(|_| LogError::TransferCountOverflow {
                count: self.transfers.len(),
            })?;
        self.seal = Some(RunCompleted::new(
            final_timestep,
            transfer_count,
            self.digest(),
        ));
        self.seal.as_ref().ok_or(LogError::AlreadySealed)
    }
    /// Returns the canonical bytes of Genesis and all currently stored Transfers.
    ///
    /// These are the exact bytes authenticated by [`Self::digest`]. The completion seal is
    /// returned separately as authentication metadata and is therefore never part of this byte
    /// sequence.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_records(&self.genesis, &self.transfers, &mut |chunk| {
            bytes.extend_from_slice(chunk);
        });
        bytes
    }
    /// Returns the digest of Genesis and all currently stored Transfers (never the seal).
    #[must_use]
    pub fn digest(&self) -> LogDigest {
        digest_records(&self.genesis, &self.transfers)
    }
    /// Rebuilds a log from decoded records while enforcing the record grammar and seal evidence.
    ///
    /// # Errors
    /// Returns [`LogError`] when Genesis is absent/repeated, ordering is invalid, or the seal is forged.
    pub fn from_records(records: impl IntoIterator<Item = Record>) -> Result<Self, LogError> {
        let mut iterator = records.into_iter();
        let genesis = match iterator.next() {
            Some(Record::Genesis(value)) => value,
            Some(_) => return Err(LogError::GenesisNotFirst),
            None => return Err(LogError::MissingGenesis),
        };
        let mut log = Self::new(genesis);
        for record in iterator {
            if log.seal.is_some() {
                return Err(LogError::RecordAfterSeal);
            }
            match record {
                Record::Genesis(_) => return Err(LogError::RepeatedGenesis),
                Record::Transfer(value) => log.append(value)?,
                Record::RunCompleted(value) => {
                    let actual_count = u64::try_from(log.transfers.len()).map_err(|_| {
                        LogError::TransferCountOverflow {
                            count: log.transfers.len(),
                        }
                    })?;
                    if value.transfer_count != actual_count {
                        return Err(LogError::SealTransferCount {
                            declared: value.transfer_count,
                            actual: actual_count,
                        });
                    }
                    let actual = log.digest();
                    if value.log_digest != actual {
                        return Err(LogError::SealDigest {
                            declared: value.log_digest,
                            actual,
                        });
                    }
                    log.seal = Some(value);
                }
            }
        }
        Ok(log)
    }
    pub fn records(&self) -> impl Iterator<Item = Record> + '_ {
        std::iter::once(Record::Genesis(self.genesis.clone()))
            .chain(self.transfers.iter().cloned().map(Record::Transfer))
            .chain(self.seal.iter().cloned().map(Record::RunCompleted))
    }
}

/// Structural failures in an authoritative record sequence.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum LogError {
    /// Fires when decoded input contains no first record.
    #[error("authoritative log is missing Genesis")]
    MissingGenesis,
    /// Fires when the first decoded record is not Genesis.
    #[error("Genesis must be the first authoritative record")]
    GenesisNotFirst,
    /// Fires when Genesis occurs after the first position.
    #[error("authoritative log contains repeated Genesis")]
    RepeatedGenesis,
    /// Fires when a record follows a terminal seal.
    #[error("no authoritative record may follow RunCompleted")]
    RecordAfterSeal,
    /// Fires when sealing an already sealed log.
    #[error("authoritative log is already sealed")]
    AlreadySealed,
    /// Fires when transfer timesteps decrease in log order.
    #[error("transfer timestep {next:?} precedes prior timestep {previous:?}")]
    TimestepOrder {
        previous: TimestepIndex,
        next: TimestepIndex,
    },
    /// Fires on an unrepresentable platform-sized transfer count.
    #[error("transfer count {count} does not fit the record representation")]
    TransferCountOverflow { count: usize },
    /// Fires when a seal's count differs from the preceding transfers.
    #[error("RunCompleted declares {declared} transfers but log contains {actual}")]
    SealTransferCount { declared: u64, actual: u64 },
    /// Fires when a seal does not authenticate its preceding records.
    #[error("RunCompleted digest {declared} does not match preceding log digest {actual}")]
    SealDigest {
        declared: LogDigest,
        actual: LogDigest,
    },
}

/// Whether a valid sequence proves completion or remains appendable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStatus {
    /// A valid terminal seal proves execution through this timestep.
    Complete { final_timestep: TimestepIndex },
    /// Absence of a seal leaves a valid prefix for later continuation.
    ResumablePrefix {
        last_transfer_timestep: Option<TimestepIndex>,
    },
}

/// A deterministically replayed stock state at one point in record order.
#[derive(Clone, Copy, Debug, PartialEq)]
struct QuantumParts {
    count: i64,
    remainder: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StockState {
    registry: SubstanceRegistry,
    finite: BTreeMap<(CompartmentId, SubstanceId), NonNegativeAmount>,
    boundary: BTreeMap<(CompartmentId, SubstanceId), SignedBoundaryBalance>,
    quantum_parts: BTreeMap<(CompartmentId, SubstanceId), QuantumParts>,
}
impl StockState {
    /// Queries a finite stock without conflating an unmodelled substance with zero.
    #[must_use]
    pub fn finite_stock(
        &self,
        compartment: &CompartmentId,
        substance: &SubstanceId,
    ) -> ValueState<NonNegativeAmount> {
        if !self.registry.contains(substance) {
            return ValueState::NotModelled;
        }
        self.finite
            .get(&(compartment.clone(), substance.clone()))
            .copied()
            .map_or(ValueState::Absent, ValueState::Present)
    }
    /// Queries a boundary balance without conflating an unmodelled substance with zero.
    #[must_use]
    pub fn boundary_balance(
        &self,
        account: &CompartmentId,
        substance: &SubstanceId,
    ) -> ValueState<SignedBoundaryBalance> {
        if !self.registry.contains(substance) {
            return ValueState::NotModelled;
        }
        self.boundary
            .get(&(account.clone(), substance.clone()))
            .copied()
            .map_or(ValueState::Absent, ValueState::Present)
    }
    pub(crate) fn finite_quantum_parts(
        &self,
        compartment: &CompartmentId,
        substance: &SubstanceId,
    ) -> Option<(u64, f64)> {
        let parts = self
            .quantum_parts
            .get(&(compartment.clone(), substance.clone()))?;
        let count = u64::try_from(parts.count).ok()?;
        Some((count, parts.remainder))
    }

    pub fn finite_stocks(
        &self,
    ) -> impl Iterator<Item = (&(CompartmentId, SubstanceId), &NonNegativeAmount)> {
        self.finite.iter()
    }
    pub fn boundary_balances(
        &self,
    ) -> impl Iterator<Item = (&(CompartmentId, SubstanceId), &SignedBoundaryBalance)> {
        self.boundary.iter()
    }
}

/// Exact binary64 totals used to inspect the final conservation closure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConservationTotals {
    genesis_total: f64,
    final_total: f64,
}
impl ConservationTotals {
    #[must_use]
    pub const fn genesis_total(self) -> f64 {
        self.genesis_total
    }
    #[must_use]
    pub const fn final_total(self) -> f64 {
        self.final_total
    }
    #[must_use]
    pub fn closes_bit_identically(self) -> bool {
        self.genesis_total.to_bits() == self.final_total.to_bits()
    }
}

/// Replay result, including a dense state snapshot for every declared timestep.
#[derive(Clone, Debug, PartialEq)]
pub struct Replay {
    artifact: Arc<ModelArtifact>,
    status: RunStatus,
    final_state: StockState,
    by_timestep: BTreeMap<TimestepIndex, StockState>,
}
impl Replay {
    #[must_use]
    pub fn artifact(&self) -> &ModelArtifact {
        &self.artifact
    }
    #[must_use]
    pub const fn status(&self) -> RunStatus {
        self.status
    }
    #[must_use]
    pub fn final_state(&self) -> &StockState {
        &self.final_state
    }
    #[must_use]
    pub fn state_at(&self, timestep: TimestepIndex) -> ValueState<&StockState> {
        if !self.artifact.horizon().contains(timestep) {
            ValueState::Absent
        } else {
            self.by_timestep
                .get(&timestep)
                .map_or(ValueState::Absent, ValueState::Present)
        }
    }

    /// Reduces finite stocks and signed boundary balances in canonical endpoint order.
    pub fn conservation_totals(
        &self,
        substance: &SubstanceId,
    ) -> Result<ValueState<ConservationTotals>, ReplayError> {
        if !self.artifact.registry().contains(substance) {
            return Ok(ValueState::NotModelled);
        }
        let initial = seed_state(&self.artifact)?;
        Ok(ValueState::Present(ConservationTotals {
            genesis_total: reduce_state_total(&initial, &self.artifact, substance)?,
            final_total: reduce_state_total(&self.final_state, &self.artifact, substance)?,
        }))
    }
}

/// Resolves Genesis' artifact and performs compatibility, seal, and deterministic fold checks.
pub struct CompletenessReader;
impl CompletenessReader {
    /// Reads using an archive that retains historical artifacts by digest.
    ///
    /// # Errors
    /// Returns [`ReplayError`] for missing/incompatible artifacts or invalid transfers/seals.
    pub fn read(
        log: &AuthoritativeLog,
        archive: &ModelArtifactArchive,
    ) -> Result<Replay, ReplayError> {
        let digest = log.genesis.model_digest;
        let artifact = archive
            .get(&digest)
            .ok_or(ReplayError::ModelArtifactNotFound { digest })?;
        replay(log, artifact)
    }
    /// Reads against an explicitly supplied artifact, rejecting any digest mismatch.
    ///
    /// # Errors
    /// Returns [`ReplayError::ModelDigestMismatch`] before attempting reconciliation.
    pub fn read_with_artifact(
        log: &AuthoritativeLog,
        artifact: Arc<ModelArtifact>,
    ) -> Result<Replay, ReplayError> {
        replay(log, artifact)
    }

    /// Reads a log only when a valid completion seal proves the full horizon.
    ///
    /// # Errors
    /// Returns [`ReplayError::MissingCompletionSeal`] for a valid resumable prefix.
    pub fn read_completed(
        log: &AuthoritativeLog,
        archive: &ModelArtifactArchive,
    ) -> Result<Replay, ReplayError> {
        let replay = Self::read(log, archive)?;
        if matches!(replay.status(), RunStatus::ResumablePrefix { .. }) {
            return Err(ReplayError::MissingCompletionSeal);
        }
        Ok(replay)
    }
}

/// Replays against a borrowed artifact without requiring an archive.
///
/// # Errors
/// Returns [`ReplayError`] for binding, record-domain, numerical, or seal failures.
pub fn replay_with_artifact(
    log: &AuthoritativeLog,
    artifact: &ModelArtifact,
) -> Result<Replay, ReplayError> {
    replay(log, Arc::new(artifact.clone()))
}

fn replay(log: &AuthoritativeLog, artifact: Arc<ModelArtifact>) -> Result<Replay, ReplayError> {
    if log.genesis.model_digest != artifact.digest() {
        return Err(ReplayError::ModelDigestMismatch {
            genesis: log.genesis.model_digest,
            supplied: artifact.digest(),
        });
    }
    if log.genesis.numerical_semantics != artifact.versions().numerical_semantics() {
        return Err(ReplayError::NumericalSemanticsMismatch);
    }
    if log.genesis.initial_stocks.topology() != artifact.topology() {
        return Err(ReplayError::GenesisTopologyMismatch);
    }
    if log.genesis.initial_stocks.registry() != artifact.registry() {
        return Err(ReplayError::GenesisRegistryMismatch);
    }
    if log.genesis.initial_stocks != *artifact.initial_stocks() {
        return Err(ReplayError::GenesisStocksMismatch);
    }
    if let Some(seal) = &log.seal {
        if seal.final_timestep != artifact.horizon().last() {
            return Err(ReplayError::SealFinalTimestep {
                declared: seal.final_timestep,
                expected: artifact.horizon().last(),
            });
        }
        let actual_count =
            u64::try_from(log.transfers.len()).map_err(|_| ReplayError::TransferCountOverflow {
                count: log.transfers.len(),
            })?;
        if seal.transfer_count != actual_count {
            return Err(ReplayError::SealTransferCount {
                declared: seal.transfer_count,
                actual: actual_count,
            });
        }
        let actual = log.digest();
        if seal.log_digest != actual {
            return Err(ReplayError::SealDigest {
                declared: seal.log_digest,
                actual,
            });
        }
    }
    let mut state = seed_state(&artifact)?;
    let mut by_timestep = BTreeMap::new();
    let mut cursor = 0_usize;
    let horizon = artifact.horizon();
    let covered_last = log
        .seal
        .as_ref()
        .map(|_| horizon.last())
        .or_else(|| log.transfers.last().map(Transfer::timestep));
    if let Some(covered_last) = covered_last {
        if !horizon.contains(covered_last) {
            return Err(ReplayError::TimestepOutsideHorizon {
                timestep: covered_last,
                first: horizon.first(),
                last: horizon.last(),
            });
        }
        let mut ordinal = horizon.first().value();
        loop {
            let timestep = TimestepIndex::new(ordinal);
            while cursor < log.transfers.len() && log.transfers[cursor].timestep == timestep {
                apply_transfer(&mut state, &log.transfers[cursor], &artifact)?;
                cursor += 1;
            }
            by_timestep.insert(timestep, state.clone());
            if timestep == covered_last {
                break;
            }
            ordinal = ordinal
                .checked_add(1)
                .ok_or(ReplayError::HorizonIterationOverflow { timestep })?;
        }
    }
    if let Some(transfer) = log.transfers.get(cursor) {
        return Err(ReplayError::TimestepOutsideHorizon {
            timestep: transfer.timestep,
            first: horizon.first(),
            last: horizon.last(),
        });
    }
    let status = log.seal.as_ref().map_or_else(
        || RunStatus::ResumablePrefix {
            last_transfer_timestep: log.transfers.last().map(Transfer::timestep),
        },
        |seal| RunStatus::Complete {
            final_timestep: seal.final_timestep,
        },
    );
    Ok(Replay {
        artifact,
        status,
        final_state: state,
        by_timestep,
    })
}

fn seed_state(artifact: &ModelArtifact) -> Result<StockState, ReplayError> {
    let mut finite = BTreeMap::new();
    let mut boundary = BTreeMap::new();
    let mut quantum_parts = BTreeMap::new();
    for endpoint in artifact.topology().endpoints() {
        for substance in artifact.registry().iter() {
            let key = (endpoint.id().clone(), substance.clone());
            let quantum =
                artifact
                    .quantum(substance)
                    .ok_or_else(|| ReplayError::ConservationReduction {
                        substance: substance.clone(),
                    })?;
            match endpoint {
                TopologyEndpoint::Finite(_) => {
                    let amount = match artifact
                        .initial_stocks()
                        .amount(endpoint.id(), substance)
                        .map_err(|_| ReplayError::GenesisStocksMismatch)?
                    {
                        ValueState::Present(value) => value,
                        _ => return Err(ReplayError::GenesisRegistryMismatch),
                    };
                    let (count, remainder) = quantum.split(amount.value()).ok_or_else(|| {
                        ReplayError::NonFiniteFold {
                            compartment: endpoint.id().clone(),
                            substance: substance.clone(),
                            timestep: artifact.horizon().first(),
                        }
                    })?;
                    let count = i64::try_from(count).map_err(|_| ReplayError::NonFiniteFold {
                        compartment: endpoint.id().clone(),
                        substance: substance.clone(),
                        timestep: artifact.horizon().first(),
                    })?;
                    finite.insert(key.clone(), amount);
                    quantum_parts.insert(key, QuantumParts { count, remainder });
                }
                TopologyEndpoint::Boundary(_) => {
                    boundary.insert(key.clone(), SignedBoundaryBalance::ZERO);
                    quantum_parts.insert(
                        key,
                        QuantumParts {
                            count: 0,
                            remainder: 0.0,
                        },
                    );
                }
            }
        }
    }
    Ok(StockState {
        registry: artifact.registry().clone(),
        finite,
        boundary,
        quantum_parts,
    })
}

fn apply_transfer(
    state: &mut StockState,
    transfer: &Transfer,
    artifact: &ModelArtifact,
) -> Result<(), ReplayError> {
    let mut staged = state.clone();
    apply_transfer_uncommitted(&mut staged, transfer, artifact)?;
    for (substance, _) in transfer.amounts.iter() {
        let before = reduce_state_total(state, artifact, substance)?;
        let after = reduce_state_total(&staged, artifact, substance)?;
        if before.to_bits() != after.to_bits() {
            return Err(ReplayError::NumericalConservationLoss {
                substance: substance.clone(),
                timestep: transfer.timestep,
                before_bits: before.to_bits(),
                after_bits: after.to_bits(),
            });
        }
    }
    *state = staged;
    Ok(())
}

fn reduce_state_total(
    state: &StockState,
    artifact: &ModelArtifact,
    substance: &SubstanceId,
) -> Result<f64, ReplayError> {
    let semantics = artifact.versions().numerical_semantics();
    let quantum =
        artifact
            .quantum(substance)
            .ok_or_else(|| ReplayError::ConservationReduction {
                substance: substance.clone(),
            })?;
    let mut count = 0_i128;
    let mut remainder = 0.0;
    for endpoint in artifact.topology().endpoints() {
        let parts = state
            .quantum_parts
            .get(&(endpoint.id().clone(), substance.clone()))
            .ok_or_else(|| ReplayError::ConservationReduction {
                substance: substance.clone(),
            })?;
        count = count.checked_add(i128::from(parts.count)).ok_or_else(|| {
            ReplayError::ConservationReduction {
                substance: substance.clone(),
            }
        })?;
        remainder = semantics.add(remainder, parts.remainder).map_err(|_| {
            ReplayError::ConservationReduction {
                substance: substance.clone(),
            }
        })?;
    }
    let count = i64::try_from(count).map_err(|_| ReplayError::ConservationReduction {
        substance: substance.clone(),
    })?;
    if count.unsigned_abs() as f64 > MAX_EXACT_WHOLE_MULTIPLES {
        return Err(ReplayError::ConservationReduction {
            substance: substance.clone(),
        });
    }
    let whole = semantics
        .multiply(count as f64, quantum.value())
        .map_err(|_| ReplayError::ConservationReduction {
            substance: substance.clone(),
        })?;
    semantics
        .add(whole, remainder)
        .map_err(|_| ReplayError::ConservationReduction {
            substance: substance.clone(),
        })
}

fn apply_transfer_uncommitted(
    state: &mut StockState,
    transfer: &Transfer,
    artifact: &ModelArtifact,
) -> Result<(), ReplayError> {
    let horizon = artifact.horizon();
    if !horizon.contains(transfer.timestep) {
        return Err(ReplayError::TimestepOutsideHorizon {
            timestep: transfer.timestep,
            first: horizon.first(),
            last: horizon.last(),
        });
    }
    validate_endpoint(transfer.source(), artifact.topology())?;
    validate_endpoint(transfer.target(), artifact.topology())?;
    if transfer.source.id() == transfer.target.id() {
        return Err(ReplayError::SelfTransfer {
            compartment: transfer.source.id().clone(),
            timestep: transfer.timestep,
        });
    }
    let connected = artifact.topology().connections().iter().any(|connection| {
        connection.source() == transfer.source.id() && connection.target() == transfer.target.id()
    });
    if !connected {
        return Err(ReplayError::UndeclaredConnection {
            connection_source: transfer.source.id().clone(),
            target: transfer.target.id().clone(),
            timestep: transfer.timestep,
        });
    }
    if transfer.amounts.registry() != artifact.registry() {
        return Err(ReplayError::TransferRegistryMismatch {
            timestep: transfer.timestep,
        });
    }
    for (substance, amount) in transfer.amounts.iter() {
        let quantum =
            artifact
                .quantum(substance)
                .ok_or_else(|| ReplayError::ConservationReduction {
                    substance: substance.clone(),
                })?;
        let count =
            quantum
                .whole_count(amount.value())
                .ok_or_else(|| ReplayError::NonQuantumTransfer {
                    substance: substance.clone(),
                    timestep: transfer.timestep,
                    amount_bits: amount.value().to_bits(),
                    quantum_bits: quantum.value().to_bits(),
                })?;
        let count = i64::try_from(count).map_err(|_| ReplayError::NonFiniteFold {
            compartment: transfer.source.id().clone(),
            substance: substance.clone(),
            timestep: transfer.timestep,
        })?;
        let source_key = (transfer.source.id().clone(), substance.clone());
        let target_key = (transfer.target.id().clone(), substance.clone());
        let source_parts = state
            .quantum_parts
            .get(&source_key)
            .copied()
            .ok_or_else(|| ReplayError::EndpointKindMismatch {
                compartment: transfer.source.id().clone(),
            })?;
        if transfer.source.is_finite() && count > source_parts.count {
            let available = state.finite.get(&source_key).copied().ok_or_else(|| {
                ReplayError::EndpointKindMismatch {
                    compartment: transfer.source.id().clone(),
                }
            })?;
            return Err(ReplayError::Overdraw {
                compartment: transfer.source.id().clone(),
                substance: substance.clone(),
                timestep: transfer.timestep,
                available_bits: available.value().to_bits(),
                requested_bits: amount.value().to_bits(),
            });
        }
        let target_parts = state
            .quantum_parts
            .get(&target_key)
            .copied()
            .ok_or_else(|| ReplayError::EndpointKindMismatch {
                compartment: transfer.target.id().clone(),
            })?;
        let next_source = QuantumParts {
            count: source_parts.count.checked_sub(count).ok_or_else(|| {
                ReplayError::NonFiniteFold {
                    compartment: transfer.source.id().clone(),
                    substance: substance.clone(),
                    timestep: transfer.timestep,
                }
            })?,
            remainder: source_parts.remainder,
        };
        let next_target = QuantumParts {
            count: target_parts.count.checked_add(count).ok_or_else(|| {
                ReplayError::NonFiniteFold {
                    compartment: transfer.target.id().clone(),
                    substance: substance.clone(),
                    timestep: transfer.timestep,
                }
            })?,
            remainder: target_parts.remainder,
        };
        state.quantum_parts.insert(source_key.clone(), next_source);
        state.quantum_parts.insert(target_key.clone(), next_target);
        let semantics = artifact.versions().numerical_semantics();
        refresh_endpoint(
            state,
            &source_key,
            transfer.source(),
            quantum,
            semantics,
            transfer,
            substance,
        )?;
        refresh_endpoint(
            state,
            &target_key,
            transfer.target(),
            quantum,
            semantics,
            transfer,
            substance,
        )?;
    }
    Ok(())
}

fn refresh_endpoint(
    state: &mut StockState,
    key: &(CompartmentId, SubstanceId),
    endpoint: &TransferEndpoint,
    quantum: crate::model_artifact::Quantum,
    semantics: NumericalSemanticsVersion,
    transfer: &Transfer,
    substance: &SubstanceId,
) -> Result<(), ReplayError> {
    let parts =
        state
            .quantum_parts
            .get(key)
            .copied()
            .ok_or_else(|| ReplayError::EndpointKindMismatch {
                compartment: endpoint.id().clone(),
            })?;
    if parts.count.unsigned_abs() as f64 > MAX_EXACT_WHOLE_MULTIPLES {
        return Err(ReplayError::NonFiniteFold {
            compartment: endpoint.id().clone(),
            substance: substance.clone(),
            timestep: transfer.timestep,
        });
    }
    let whole = semantics
        .multiply(parts.count as f64, quantum.value())
        .map_err(|_| ReplayError::NonFiniteFold {
            compartment: endpoint.id().clone(),
            substance: substance.clone(),
            timestep: transfer.timestep,
        })?;
    let value = semantics
        .add(whole, parts.remainder)
        .map_err(|_| ReplayError::NonFiniteFold {
            compartment: endpoint.id().clone(),
            substance: substance.clone(),
            timestep: transfer.timestep,
        })?;
    match endpoint {
        TransferEndpoint::Finite(_) => {
            let amount =
                NonNegativeAmount::try_from(value).map_err(|_| ReplayError::NonFiniteFold {
                    compartment: endpoint.id().clone(),
                    substance: substance.clone(),
                    timestep: transfer.timestep,
                })?;
            state.finite.insert(key.clone(), amount);
        }
        TransferEndpoint::Boundary(_) => {
            let balance =
                SignedBoundaryBalance::try_from(value).map_err(|_| ReplayError::NonFiniteFold {
                    compartment: endpoint.id().clone(),
                    substance: substance.clone(),
                    timestep: transfer.timestep,
                })?;
            state.boundary.insert(key.clone(), balance);
        }
    }
    Ok(())
}
fn validate_endpoint(endpoint: &TransferEndpoint, topology: &Topology) -> Result<(), ReplayError> {
    match (endpoint, topology.endpoint(endpoint.id())) {
        (_, None) => Err(ReplayError::UnknownEndpoint {
            compartment: endpoint.id().clone(),
        }),
        (TransferEndpoint::Finite(_), Some(TopologyEndpoint::Finite(_)))
        | (TransferEndpoint::Boundary(_), Some(TopologyEndpoint::Boundary(_))) => Ok(()),
        _ => Err(ReplayError::EndpointKindMismatch {
            compartment: endpoint.id().clone(),
        }),
    }
}

/// Failures which prevent exact, compatible replay.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum ReplayError {
    /// Fires when a modelled substance cannot be reduced without missing or non-finite state.
    #[error("cannot reduce conservation total for modelled substance `{substance}`")]
    ConservationReduction { substance: SubstanceId },
    /// Fires when binary64 endpoint updates change the canonical conserved total.
    #[error(
        "transfer at {timestep:?} changes conserved total for `{substance}` from bits {before_bits:#018x} to {after_bits:#018x}"
    )]
    NumericalConservationLoss {
        substance: SubstanceId,
        timestep: TimestepIndex,
        before_bits: u64,
        after_bits: u64,
    },
    /// Fires when a transfer amount is not the exact binary64 image of whole declared quanta.
    #[error(
        "transfer at {timestep:?} carries non-quantized amount bits {amount_bits:#018x} for `{substance}` under quantum bits {quantum_bits:#018x}"
    )]
    NonQuantumTransfer {
        substance: SubstanceId,
        timestep: TimestepIndex,
        amount_bits: u64,
        quantum_bits: u64,
    },
    /// Fires when a caller explicitly requires a completed run but receives a prefix.
    #[error("authoritative log is a resumable prefix without RunCompleted")]
    MissingCompletionSeal,
    /// Fires when Genesis names a digest absent from the archive.
    #[error("model artifact `{digest}` required by Genesis is not retained")]
    ModelArtifactNotFound { digest: ModelDigest },
    /// Fires when replay is attempted with content other than the Genesis-bound artifact.
    #[error("model digest mismatch: Genesis binds `{genesis}`, supplied artifact is `{supplied}`")]
    ModelDigestMismatch {
        genesis: ModelDigest,
        supplied: ModelDigest,
    },
    /// Fires when Genesis' numerical version differs from its artifact.
    #[error("Genesis numerical-semantics version differs from the bound model artifact")]
    NumericalSemanticsMismatch,
    /// Fires when Genesis initial stocks differ from the content-addressed artifact.
    #[error("Genesis initial stocks differ from the bound model artifact")]
    GenesisStocksMismatch,
    /// Fires when Genesis' initial-stock registry differs from the artifact registry.
    #[error("Genesis substance registry differs from the bound model artifact")]
    GenesisRegistryMismatch,
    /// Fires when Genesis' topology differs from the artifact topology.
    #[error("Genesis topology differs from the bound model artifact")]
    GenesisTopologyMismatch,
    /// Fires when a transfer occurs outside the declared horizon.
    #[error("transfer timestep {timestep:?} lies outside declared horizon {first:?}..={last:?}")]
    TimestepOutsideHorizon {
        timestep: TimestepIndex,
        first: TimestepIndex,
        last: TimestepIndex,
    },
    /// Fires when an endpoint is absent from the bound topology.
    #[error("transfer endpoint `{compartment}` is absent from the bound topology")]
    UnknownEndpoint { compartment: CompartmentId },
    /// Fires when a transfer's structural endpoint type disagrees with topology.
    #[error("transfer endpoint `{compartment}` has the wrong structural kind")]
    EndpointKindMismatch { compartment: CompartmentId },
    /// Fires when no bound directed connection permits the transfer.
    #[error(
        "transfer from `{connection_source}` to `{target}` at {timestep:?} is not a declared connection"
    )]
    UndeclaredConnection {
        connection_source: CompartmentId,
        target: CompartmentId,
        timestep: TimestepIndex,
    },
    /// Fires when a transfer names one endpoint twice.
    #[error("self-transfer at compartment `{compartment}` at {timestep:?} is invalid")]
    SelfTransfer {
        compartment: CompartmentId,
        timestep: TimestepIndex,
    },
    /// Fires when amounts are bound to a registry other than this run's registry.
    #[error("transfer at {timestep:?} is bound to a different substance registry")]
    TransferRegistryMismatch { timestep: TimestepIndex },
    /// Fires before appending/applying a debit larger than finite stock.
    #[error(
        "overdraw of compartment `{compartment}`, substance `{substance}` at {timestep:?}: available bits {available_bits:#018x}, requested bits {requested_bits:#018x}"
    )]
    Overdraw {
        compartment: CompartmentId,
        substance: SubstanceId,
        timestep: TimestepIndex,
        available_bits: u64,
        requested_bits: u64,
    },
    /// Fires when fold arithmetic produces an unrepresentable amount.
    #[error(
        "non-finite fold result for compartment `{compartment}`, substance `{substance}` at {timestep:?}"
    )]
    NonFiniteFold {
        compartment: CompartmentId,
        substance: SubstanceId,
        timestep: TimestepIndex,
    },
    /// Fires when the inclusive horizon cannot advance.
    #[error("cannot advance declared horizon after {timestep:?}")]
    HorizonIterationOverflow { timestep: TimestepIndex },
    /// Fires when a valid seal does not name the artifact's declared last timestep.
    #[error("RunCompleted final timestep {declared:?} differs from model horizon end {expected:?}")]
    SealFinalTimestep {
        declared: TimestepIndex,
        expected: TimestepIndex,
    },
    /// Fires when a seal count differs from its preceding transfers.
    #[error("RunCompleted declares {declared} transfers but log contains {actual}")]
    SealTransferCount { declared: u64, actual: u64 },
    /// Fires when a seal digest differs from its preceding records.
    #[error("RunCompleted digest {declared} does not match preceding log digest {actual}")]
    SealDigest {
        declared: LogDigest,
        actual: LogDigest,
    },
    /// Fires when an in-memory transfer count cannot be represented in a seal.
    #[error("transfer count {count} cannot be represented in RunCompleted")]
    TransferCountOverflow { count: usize },
}

/// Returns the incidence coefficients for a Transfer; their sum is structurally zero.
pub fn incidence_column(transfer: &Transfer) -> [(TransferEndpoint, i8); 2] {
    [(transfer.source.clone(), -1), (transfer.target.clone(), 1)]
}
/// Verifies the structural conservation law for every transfer column.
#[must_use]
pub fn incidence_columns_close(log: &AuthoritativeLog) -> bool {
    log.transfers.iter().all(|transfer| {
        incidence_column(transfer)
            .iter()
            .map(|(_, coefficient)| i16::from(*coefficient))
            .sum::<i16>()
            == 0
    })
}

fn digest_records(genesis: &Genesis, transfers: &[Transfer]) -> LogDigest {
    let mut hasher = Sha256::new();
    write_records(genesis, transfers, &mut |chunk| hasher.update(chunk));
    LogDigest(hasher.finalize().into())
}

fn write_records(genesis: &Genesis, transfers: &[Transfer], sink: &mut impl FnMut(&[u8])) {
    sink(b"incidence:authoritative-log:v1\0");
    sink(genesis.run_id.as_bytes());
    sink(genesis.model_digest.as_bytes());
    sink(&[version_byte(genesis.numerical_semantics)]);
    write_initial(sink, &genesis.initial_stocks);
    sink(&(transfers.len() as u64).to_be_bytes());
    for transfer in transfers {
        sink(&transfer.timestep.value().to_be_bytes());
        write_endpoint(sink, &transfer.source);
        write_endpoint(sink, &transfer.target);
        write_registry(sink, transfer.amounts.registry());
        let entries = transfer.amounts.iter().collect::<Vec<_>>();
        sink(&(entries.len() as u64).to_be_bytes());
        for (substance, amount) in entries {
            write_string(sink, substance.as_str());
            sink(&amount.value().to_bits().to_be_bytes());
        }
    }
}
fn version_byte(value: NumericalSemanticsVersion) -> u8 {
    match value {
        NumericalSemanticsVersion::V1 => 1,
        NumericalSemanticsVersion::V2 => 2,
    }
}
fn write_initial(sink: &mut impl FnMut(&[u8]), initial: &InitialStocks) {
    let endpoints = initial.topology().endpoints().collect::<Vec<_>>();
    sink(&(endpoints.len() as u64).to_be_bytes());
    for endpoint in endpoints {
        sink(&[if matches!(endpoint, TopologyEndpoint::Finite(_)) {
            0
        } else {
            1
        }]);
        write_string(sink, endpoint.id().as_str());
    }
    let connections = initial.topology().connections();
    sink(&(connections.len() as u64).to_be_bytes());
    for connection in connections {
        write_string(sink, connection.source().as_str());
        write_string(sink, connection.target().as_str());
    }
    write_registry(sink, initial.registry());
    let entries = initial.iter().collect::<Vec<_>>();
    sink(&(entries.len() as u64).to_be_bytes());
    for (compartment, vector) in entries {
        write_string(sink, compartment.as_str());
        let amounts = vector.iter().collect::<Vec<_>>();
        sink(&(amounts.len() as u64).to_be_bytes());
        for (substance, amount) in amounts {
            write_string(sink, substance.as_str());
            sink(&amount.value().to_bits().to_be_bytes());
        }
    }
}
fn write_registry(sink: &mut impl FnMut(&[u8]), registry: &SubstanceRegistry) {
    sink(&(registry.len() as u64).to_be_bytes());
    for substance in registry.iter() {
        write_string(sink, substance.as_str());
    }
}
fn write_endpoint(sink: &mut impl FnMut(&[u8]), endpoint: &TransferEndpoint) {
    sink(&[if endpoint.is_finite() { 0 } else { 1 }]);
    write_string(sink, endpoint.id().as_str());
}
fn write_string(sink: &mut impl FnMut(&[u8]), value: &str) {
    sink(&(value.len() as u64).to_be_bytes());
    sink(value.as_bytes());
}
