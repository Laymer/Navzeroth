//! Manual local parties reference exact profiles, never merge character state.
use crate::{
    ProfileKey, SqliteStore,
    error::{SqliteResult, invalid, storage},
    sqlite::find_profile,
};
use azeroth_core::{error::Result, types::PartyId};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::collections::BTreeSet;

pub const MAX_PARTIES: usize = 128;
pub const MAX_PARTICIPANTS: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyProfile {
    pub id: PartyId,
    /// Ordered membership. Identity survives deletion; a missing snapshot stays missing.
    pub participants: Vec<ProfileKey>,
}

fn validate(party: &PartyProfile) -> Result<()> {
    if party.participants.is_empty() || party.participants.len() > MAX_PARTICIPANTS {
        return Err(invalid("Party participant count is out of range"));
    }
    let first = &party.participants[0];
    let mut characters = BTreeSet::new();
    for key in &party.participants {
        if key.context != first.context || key.fixture != first.fixture {
            return Err(invalid("Party profiles require matching data contexts"));
        }
        if !characters.insert(&key.character) {
            return Err(invalid("Party participants must be distinct characters"));
        }
    }
    Ok(())
}

impl SqliteStore {
    /// Atomically create/replace a manual party using existing local profiles.
    /// Does not change character snapshots, preferences or import history.
    pub fn save_party(&mut self, party: &PartyProfile) -> Result<()> {
        validate(party)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM party_profiles WHERE party_id = ?1)",
                [party.id.as_str()],
                |r| r.get(0),
            )
            .db()?;
        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM party_profiles", [], |r| r.get(0))
            .db()?;
        if !exists && count >= MAX_PARTIES as i64 {
            return Err(invalid("Party limit reached"));
        }
        // Resolve everything before replacing the membership; rollback on any error.
        let members = party
            .participants
            .iter()
            .map(|key| {
                let (row, _) = find_profile(&tx, key)?
                    .ok_or_else(|| invalid("No such party participant profile"))?;
                let document = serde_json::to_string(key)
                    .map_err(|_| storage("Profile identity cannot be encoded"))?;
                Ok((row, document))
            })
            .collect::<Result<Vec<_>>>()?;
        tx.execute(
            "INSERT INTO party_profiles (party_id) VALUES (?1) ON CONFLICT DO NOTHING",
            [party.id.as_str()],
        )
        .db()?;
        tx.execute(
            "DELETE FROM party_participants WHERE party_id = ?1",
            [party.id.as_str()],
        )
        .db()?;
        for (position, (profile, document)) in members.iter().enumerate() {
            tx.execute("INSERT INTO party_participants (party_id, position, profile_id, profile_key) VALUES (?1, ?2, ?3, ?4)", params![party.id.as_str(), position as i64, profile, document]).db()?;
        }
        tx.commit().db()
    }

    pub fn party(&self, id: &PartyId) -> Result<Option<PartyProfile>> {
        let exists: Option<String> = self
            .conn
            .query_row(
                "SELECT party_id FROM party_profiles WHERE party_id = ?1",
                [id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .db()?;
        if exists.is_none() {
            return Ok(None);
        }
        let mut query = self.conn.prepare("SELECT profile_key FROM party_participants WHERE party_id = ?1 ORDER BY position LIMIT ?2").db()?;
        let rows = query
            .query_map(params![id.as_str(), (MAX_PARTICIPANTS + 1) as i64], |r| {
                r.get::<_, String>(0)
            })
            .db()?;
        let mut participants = Vec::new();
        for row in rows {
            participants.push(
                serde_json::from_str(&row.db()?)
                    .map_err(|_| storage("Stored party identity is invalid"))?,
            );
        }
        let party = PartyProfile {
            id: id.clone(),
            participants,
        };
        validate(&party).map_err(|_| storage("Stored party failed validation"))?;
        Ok(Some(party))
    }

    pub fn party_ids(&self) -> Result<Vec<PartyId>> {
        let mut query = self
            .conn
            .prepare("SELECT party_id FROM party_profiles ORDER BY party_id LIMIT ?1")
            .db()?;
        let rows = query
            .query_map([MAX_PARTIES as i64], |r| r.get::<_, String>(0))
            .db()?;
        rows.map(|row| {
            PartyId::new(row.db()?).map_err(|_| storage("Stored party identity is invalid"))
        })
        .collect()
    }

    /// Delete only this party configuration; character data remains intact.
    pub fn delete_party(&mut self, id: &PartyId) -> Result<bool> {
        Ok(self
            .conn
            .execute(
                "DELETE FROM party_profiles WHERE party_id = ?1",
                [id.as_str()],
            )
            .db()?
            != 0)
    }
}
