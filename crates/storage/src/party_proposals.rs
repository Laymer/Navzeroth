//! One bounded local proposal per party. Every mutation uses a revision guard,
//! including acceptances, so another process cannot silently replace consent.
use crate::{
    SqliteStore,
    error::{SqliteResult, invalid, storage},
};
use azeroth_core::{
    error::Result,
    types::{PartyId, Sha256Hex, UtcMillis},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const MAX_PROPOSAL_BYTES: usize = 2 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalGuard {
    pub proposal_id: Sha256Hex,
    pub revision: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalAcceptance {
    pub accepted_at_utc_ms: Option<UtcMillis>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyProposal {
    pub schema_version: u32,
    pub party_id: PartyId,
    pub guard: ProposalGuard,
    pub proposer_profile_id: String,
    /// Bounded application data, never executable code or transport instructions.
    pub request: String,
    pub result_hash: Sha256Hex,
    pub selected_objectives: Vec<azeroth_core::types::ObjectiveId>,
    pub expected_total_ms: azeroth_core::types::Millis,
    pub context: azeroth_core::types::DataContext,
    pub fixture: bool,
    pub created_at_utc_ms: Option<UtcMillis>,
    pub acceptances: BTreeMap<String, LocalAcceptance>,
}
fn validate(proposal: &PartyProposal) -> Result<()> {
    if proposal.schema_version != 1
        || proposal.guard.revision == 0
        || proposal.proposer_profile_id.len() > 128
        || proposal.proposer_profile_id.is_empty()
        || proposal.request.len() > 1024 * 1024
        || proposal.acceptances.len() > 40
        || proposal.selected_objectives.is_empty()
        || proposal.selected_objectives.len() > 32
        || proposal
            .acceptances
            .keys()
            .any(|id| id.is_empty() || id.len() > 128)
    {
        return Err(invalid("Local proposal bounds or schema are invalid"));
    }
    // Syntax check only. Application reads validate the typed guarded request.
    serde_json::from_str::<serde_json::Value>(&proposal.request)
        .map_err(|_| invalid("Local proposal request is invalid data"))?;
    Ok(())
}
fn encode(proposal: &PartyProposal) -> Result<String> {
    validate(proposal)?;
    let document =
        serde_json::to_string(proposal).map_err(|_| storage("Cannot encode local proposal"))?;
    if document.len() > MAX_PROPOSAL_BYTES {
        return Err(invalid("Local proposal document exceeds size limit"));
    }
    Ok(document)
}
impl SqliteStore {
    pub fn party_proposal(&self, party: &PartyId) -> Result<Option<PartyProposal>> {
        let row: Option<(String, u32, Option<String>)> = self.conn.query_row(
            "SELECT proposal_id, revision, CASE WHEN length(CAST(document AS BLOB))<=?2 THEN document ELSE NULL END FROM party_plan_proposals WHERE party_id=?1",
            params![party.as_str(),MAX_PROPOSAL_BYTES as i64], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))
            .optional().db()?;
        row.map(|(id, revision, text)| {
            let text = text.ok_or_else(|| storage("Stored local proposal exceeds size limit"))?;
            let proposal: PartyProposal = serde_json::from_str(&text)
                .map_err(|_| storage("Stored local proposal is invalid"))?;
            validate(&proposal)?;
            if &proposal.party_id != party
                || proposal.guard.proposal_id.as_str() != id
                || proposal.guard.revision != revision
            {
                return Err(storage("Stored local proposal identity is inconsistent"));
            }
            Ok(proposal)
        })
        .transpose()
    }
    pub fn save_party_proposal(
        &mut self,
        proposal: &PartyProposal,
        expected: Option<&ProposalGuard>,
    ) -> Result<()> {
        let text = encode(proposal)?;
        let expected_revision = expected
            .map_or(Some(1), |g| g.revision.checked_add(1))
            .ok_or_else(|| invalid("Local proposal revision overflows"))?;
        if proposal.guard.revision != expected_revision {
            return Err(invalid("Local proposal revision does not follow its guard"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let old: Option<(String, u32)> = tx
            .query_row(
                "SELECT proposal_id, revision FROM party_plan_proposals WHERE party_id=?1",
                [proposal.party_id.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .db()?;
        let valid = match (old.as_ref(), expected) {
            (None, None) => true,
            (Some((id, revision)), Some(guard)) => {
                id == guard.proposal_id.as_str() && *revision == guard.revision
            }
            _ => false,
        };
        if !valid {
            return Err(invalid(
                "Local proposal changed; refresh before changing agreement",
            ));
        }
        tx.execute("INSERT INTO party_plan_proposals(party_id,proposal_id,revision,document) VALUES(?1,?2,?3,?4) ON CONFLICT(party_id) DO UPDATE SET proposal_id=excluded.proposal_id,revision=excluded.revision,document=excluded.document", params![proposal.party_id.as_str(),proposal.guard.proposal_id.as_str(),proposal.guard.revision,text]).db()?;
        tx.commit().db()
    }
}
