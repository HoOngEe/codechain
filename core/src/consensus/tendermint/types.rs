// Copyright 2018-2019 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::fmt;
use std::ops::{Deref, DerefMut};

use ckey::SchnorrSignature;
use ctypes::BlockHash;
use primitives::Bytes;
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};

use super::super::BitSet;
use super::message::VoteStep;
use crate::block::{IsBlock, SealedBlock};
use crate::consensus::{sortition::seed::SeedInfo, sortition::PriorityMessage, Priority};

pub type Height = u64;
pub type View = u64;

#[derive(Clone)]
pub enum TendermintState {
    Propose,
    ProposeWaitBlockGeneration {
        parent_hash: BlockHash,
    },
    ProposeWaitImported {
        block: Box<SealedBlock>,
    },
    Prevote,
    Precommit,
    Commit {
        view: View,
        block_hash: BlockHash,
    },
    CommitTimedout {
        view: View,
        block_hash: BlockHash,
    },
}

impl TendermintState {
    pub fn to_step(&self) -> Step {
        match self {
            TendermintState::Propose => Step::Propose,
            TendermintState::ProposeWaitBlockGeneration {
                ..
            } => Step::Propose,
            TendermintState::ProposeWaitImported {
                ..
            } => Step::Propose,
            TendermintState::Prevote => Step::Prevote,
            TendermintState::Precommit => Step::Precommit,
            TendermintState::Commit {
                ..
            } => Step::Commit,
            TendermintState::CommitTimedout {
                ..
            } => Step::Commit,
        }
    }

    pub fn is_commit(&self) -> bool {
        match self {
            TendermintState::Commit {
                ..
            } => true,
            TendermintState::CommitTimedout {
                ..
            } => true,
            _ => false,
        }
    }

    pub fn is_commit_timedout(&self) -> bool {
        match self {
            TendermintState::CommitTimedout {
                ..
            } => true,
            _ => false,
        }
    }

    pub fn committed(&self) -> Option<(View, BlockHash)> {
        match self {
            TendermintState::Commit {
                block_hash,
                view,
            } => Some((*view, *block_hash)),
            TendermintState::CommitTimedout {
                block_hash,
                view,
            } => Some((*view, *block_hash)),
            TendermintState::Propose => None,
            TendermintState::ProposeWaitBlockGeneration {
                ..
            } => None,
            TendermintState::ProposeWaitImported {
                ..
            } => None,
            TendermintState::Prevote => None,
            TendermintState::Precommit => None,
        }
    }
}

impl fmt::Debug for TendermintState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TendermintState::Propose => write!(f, "TendermintState::Propose"),
            TendermintState::ProposeWaitBlockGeneration {
                parent_hash,
            } => write!(f, "TendermintState::ProposeWaitBlockGeneration({})", parent_hash),
            TendermintState::ProposeWaitImported {
                block,
            } => write!(f, "TendermintState::ProposeWaitImported({})", block.header().hash()),
            TendermintState::Prevote => write!(f, "TendermintState::Prevote"),
            TendermintState::Precommit => write!(f, "TendermintState::Precommit"),
            TendermintState::Commit {
                block_hash,
                view,
            } => write!(f, "TendermintState::Commit({}, {})", block_hash, view),
            TendermintState::CommitTimedout {
                block_hash,
                view,
            } => write!(f, "TendermintState::CommitTimedout({}, {})", block_hash, view),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Step {
    Propose,
    Prevote,
    Precommit,
    Commit,
}

impl Step {
    pub fn number(self) -> u8 {
        match self {
            Step::Propose => 0,
            Step::Prevote => 1,
            Step::Precommit => 2,
            Step::Commit => 3,
        }
    }
}

impl Decodable for Step {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        match rlp.as_val()? {
            0u8 => Ok(Step::Propose),
            1 => Ok(Step::Prevote),
            2 => Ok(Step::Precommit),
            // FIXME: Step::Commit case is not necessary if Api::send_local_message does not serialize message.
            3 => Ok(Step::Commit),
            _ => Err(DecoderError::Custom("Invalid step.")),
        }
    }
}

impl Encodable for Step {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.append_single_value(&self.number());
    }
}

pub struct PeerState {
    pub vote_step: VoteStep,
    pub priority: Option<Priority>,
    pub proposal: Option<BlockHash>,
    pub messages: BitSet,
}

impl PeerState {
    pub fn new() -> Self {
        PeerState {
            vote_step: VoteStep::new(0, 0, Step::Propose),
            priority: None,
            proposal: None,
            messages: BitSet::new(),
        }
    }
}

pub struct TendermintSealView<'a> {
    seal: &'a [Bytes],
}

impl<'a> TendermintSealView<'a> {
    pub fn new(bytes: &'a [Bytes]) -> TendermintSealView<'a> {
        TendermintSealView {
            seal: bytes,
        }
    }

    /// The parent block is finalized at this view.
    /// Signatures in the seal field is signed for this view.
    pub fn parent_block_finalized_view(&self) -> Result<u64, DecoderError> {
        let view_rlp =
            self.seal.get(0).expect("block went through verify_block_basic; block has .seal_fields() fields; qed");
        Rlp::new(view_rlp.as_slice()).as_val()
    }

    /// Block is created at auth_view.
    /// Block verifier use other_view to verify the author
    pub fn author_view(&self) -> Result<u64, DecoderError> {
        let view_rlp =
            self.seal.get(1).expect("block went through verify_block_basic; block has .seal_fields() fields; qed");
        Rlp::new(view_rlp.as_slice()).as_val()
    }

    pub fn bitset(&self) -> Result<BitSet, DecoderError> {
        let view_rlp =
            self.seal.get(3).expect("block went through verify_block_basic; block has .seal_fields() fields; qed");
        Rlp::new(view_rlp.as_slice()).as_val()
    }

    pub fn precommits(&self) -> Rlp<'a> {
        Rlp::new(
            &self.seal.get(2).expect("block went through verify_block_basic; block has .seal_fields() fields; qed"),
        )
    }

    pub fn signatures(&self) -> Result<Vec<(usize, SchnorrSignature)>, DecoderError> {
        let precommits = self.precommits();
        let bitset = self.bitset()?;
        debug_assert_eq!(bitset.count(), precommits.item_count()?);

        let bitset_iter = bitset.true_index_iter();

        let signatures = precommits.iter().map(|rlp| rlp.as_val::<SchnorrSignature>());
        bitset_iter
            .zip(signatures)
            .map(|(index, signature)| signature.map(|signature| (index, signature)))
            .collect::<Result<_, _>>()
    }

    pub fn vrf_seed_info(&self) -> Result<SeedInfo, DecoderError> {
        let seed_rlp =
            self.seal.get(4).expect("block went through verify_block_basic; block has .seal_fields() fields; qed");
        Rlp::new(seed_rlp.as_slice()).as_val()
    }
}

#[derive(Copy, Clone)]
pub enum TwoThirdsMajority {
    Empty,
    Lock(View, BlockHash),
    Unlock(View),
}

impl TwoThirdsMajority {
    pub fn from_message(view: View, block_hash: Option<BlockHash>) -> Self {
        match block_hash {
            Some(block_hash) => TwoThirdsMajority::Lock(view, block_hash),
            None => TwoThirdsMajority::Unlock(view),
        }
    }

    pub fn view(&self) -> Option<View> {
        match self {
            TwoThirdsMajority::Empty => None,
            TwoThirdsMajority::Lock(view, _) => Some(*view),
            TwoThirdsMajority::Unlock(view) => Some(*view),
        }
    }

    pub fn block_hash(&self) -> Option<BlockHash> {
        match self {
            TwoThirdsMajority::Empty => None,
            TwoThirdsMajority::Lock(_, block_hash) => Some(*block_hash),
            TwoThirdsMajority::Unlock(_) => None,
        }
    }
}

/// ProposalInfo stores the information for a valid proposal
#[derive(Debug, PartialEq)]
pub struct ProposalInfo {
    block_hash: BlockHash,
    priority_message: PriorityMessage,
    block: Bytes,
    signature: SchnorrSignature,
    is_imported: bool,
}

impl ProposalInfo {
    pub fn priority_message(&self) -> &PriorityMessage {
        &self.priority_message
    }

    pub fn block(&self) -> &Bytes {
        &self.block
    }

    pub fn signature(&self) -> &SchnorrSignature {
        &self.signature
    }
}

/// Proposal stores ProposalInfo in order of priority
#[derive(Debug, PartialEq)]
pub struct Proposal(Vec<ProposalInfo>);

impl Deref for Proposal {
    type Target = Vec<ProposalInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Proposal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Proposal {
    pub fn new() -> Self {
        Proposal(Vec::new())
    }

    pub fn new_highest(
        &mut self,
        block_hash: BlockHash,
        priority_message: PriorityMessage,
        block: Bytes,
        signature: SchnorrSignature,
    ) {
        self.insert(0, ProposalInfo {
            block_hash,
            priority_message,
            block,
            signature,
            is_imported: false,
        });
    }

    pub fn get_highest_proposal_info(&self) -> Option<&ProposalInfo> {
        self.get(0)
    }

    pub fn get_highest_priority(&self) -> Option<Priority> {
        self.get(0).map(|info| info.priority_message.priority())
    }

    pub fn new_imported(&mut self, block_hash: BlockHash) {
        if let Some(mut info) = self.iter_mut().find(|info| info.block_hash == block_hash) {
            info.is_imported = true;
        }
    }

    pub fn block_hash(&self) -> Option<BlockHash> {
        self.get(0).map(|info| info.block_hash)
    }

    pub fn imported_block_hash(&self) -> Option<BlockHash> {
        self.iter().find(|&info| info.is_imported).map(|info| info.block_hash)
    }
}
