#![cfg_attr(not(feature = "std"), no_std)]

/// A pallet for blockchain-based campus voting system
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Configuration trait for the pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Maximum number of candidates allowed per election
        #[pallet::constant]
        type MaxCandidates: Get<u32>;
        
        /// Maximum length of candidate name
        #[pallet::constant]
        type MaxNameLength: Get<u32>;
    }

    // ==================== STORAGE ====================

    /// Stores election details
    #[pallet::storage]
    #[pallet::getter(fn election)]
    pub type Election<T: Config> = StorageValue<
        _,
        ElectionInfo<T>,
        OptionQuery
    >;

    /// List of all candidates in the current election
    #[pallet::storage]
    #[pallet::getter(fn candidates)]
    pub type Candidates<T: Config> = StorageValue<
        _,
        BoundedVec<Candidate<T>, T::MaxCandidates>,
        ValueQuery
    >;

    /// Track which addresses have already voted
    /// Maps: voter_address => candidate_id
    #[pallet::storage]
    #[pallet::getter(fn has_voted)]
    pub type HasVoted<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u32, // candidate_id they voted for
        OptionQuery
    >;

    /// Vote count for each candidate
    /// Maps: candidate_id => vote_count
    #[pallet::storage]
    #[pallet::getter(fn vote_count)]
    pub type VoteCount<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32, // candidate_id
        u32, // vote count
        ValueQuery
    >;

    /// Total number of votes cast
    #[pallet::storage]
    #[pallet::getter(fn total_votes)]
    pub type TotalVotes<T: Config> = StorageValue<_, u32, ValueQuery>;

    // ==================== TYPES ====================

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ElectionInfo<T: Config> {
        /// Election title
        pub title: BoundedVec<u8, T::MaxNameLength>,
        /// Election start block
        pub start_block: BlockNumberFor<T>,
        /// Election end block
        pub end_block: BlockNumberFor<T>,
        /// Is election active?
        pub is_active: bool,
        /// Is election finalized?
        pub is_finalized: bool,
    }

    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Candidate<T: Config> {
        /// Unique candidate ID
        pub id: u32,
        /// Candidate name
        pub name: BoundedVec<u8, T::MaxNameLength>,
        /// Candidate description/manifesto
        pub description: BoundedVec<u8, T::MaxNameLength>,
    }

    // ==================== EVENTS ====================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Election created [title, start_block, end_block]
        ElectionCreated {
            title: Vec<u8>,
            start_block: BlockNumberFor<T>,
            end_block: BlockNumberFor<T>,
        },
        /// Candidate added [candidate_id, name]
        CandidateAdded {
            candidate_id: u32,
            name: Vec<u8>,
        },
        /// Vote cast [voter, candidate_id]
        VoteCast {
            voter: T::AccountId,
            candidate_id: u32,
        },
        /// Election started
        ElectionStarted,
        /// Election ended
        ElectionEnded,
        /// Election results finalized
        ElectionFinalized,
        /// Election reset
        ElectionReset,
    }

    // ==================== ERRORS ====================

    #[pallet::error]
    pub enum Error<T> {
        /// Election already exists
        ElectionAlreadyExists,
        /// No election exists
        NoElectionExists,
        /// Election has not started yet
        ElectionNotStarted,
        /// Election has already ended
        ElectionEnded,
        /// Election is not active
        ElectionNotActive,
        /// Voter has already voted
        AlreadyVoted,
        /// Invalid candidate ID
        InvalidCandidate,
        /// Maximum number of candidates reached
        TooManyCandidates,
        /// Name/description too long
        NameTooLong,
        /// Invalid time range (end must be after start)
        InvalidTimeRange,
        /// Election already finalized
        AlreadyFinalized,
        /// Cannot modify active election
        ElectionIsActive,
    }

    // ==================== EXTRINSICS ====================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        
        /// Create a new election (Admin only - you'll want to add permission checks)
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000,0))]
        pub fn create_election(
            origin: OriginFor<T>,
            title: Vec<u8>,
            start_block: BlockNumberFor<T>,
            end_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            ensure_root(origin)?; // Only sudo/admin can create election
            
            ensure!(!Election::<T>::exists(), Error::<T>::ElectionAlreadyExists);
            ensure!(end_block > start_block, Error::<T>::InvalidTimeRange);
            
            let bounded_title: BoundedVec<u8, T::MaxNameLength> = 
                title.clone().try_into().map_err(|_| Error::<T>::NameTooLong)?;
            
            let election_info = ElectionInfo {
                title: bounded_title,
                start_block,
                end_block,
                is_active: false,
                is_finalized: false,
            };
            
            Election::<T>::put(election_info);
            
            Self::deposit_event(Event::ElectionCreated {
                title,
                start_block,
                end_block,
            });
            
            Ok(())
        }

        /// Add a candidate to the election (Admin only)
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000,0))]
        pub fn add_candidate(
            origin: OriginFor<T>,
            candidate_id: u32,
            name: Vec<u8>,
            description: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            
            ensure!(Election::<T>::exists(), Error::<T>::NoElectionExists);
            
            let election = Election::<T>::get().ok_or(Error::<T>::NoElectionExists)?;
            ensure!(!election.is_active, Error::<T>::ElectionIsActive);
            
            let bounded_name: BoundedVec<u8, T::MaxNameLength> = 
                name.clone().try_into().map_err(|_| Error::<T>::NameTooLong)?;
            let bounded_description: BoundedVec<u8, T::MaxNameLength> = 
                description.try_into().map_err(|_| Error::<T>::NameTooLong)?;
            
            let candidate = Candidate {
                id: candidate_id,
                name: bounded_name,
                description: bounded_description,
            };
            
            Candidates::<T>::try_mutate(|candidates| {
                candidates.try_push(candidate)
                    .map_err(|_| Error::<T>::TooManyCandidates)
            })?;
            
            // Initialize vote count for this candidate
            VoteCount::<T>::insert(candidate_id, 0u32);
            
            Self::deposit_event(Event::CandidateAdded {
                candidate_id,
                name,
            });
            
            Ok(())
        }

        /// Start the election (Admin only)
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000,0))]
        pub fn start_election(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            
            Election::<T>::try_mutate(|election_opt| {
                let election = election_opt.as_mut().ok_or(Error::<T>::NoElectionExists)?;
                
                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(current_block >= election.start_block, Error::<T>::ElectionNotStarted);
                ensure!(current_block < election.end_block, Error::<T>::ElectionEnded);
                
                election.is_active = true;
                
                Self::deposit_event(Event::ElectionStarted);
                Ok(())
            })
        }

        /// Cast a vote (Any registered student can call this)
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000,0))]
        pub fn cast_vote(
            origin: OriginFor<T>,
            candidate_id: u32,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            
            // Check election exists and is active
            let election = Election::<T>::get().ok_or(Error::<T>::NoElectionExists)?;
            ensure!(election.is_active, Error::<T>::ElectionNotActive);
            
            // Check we're within voting period
            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(current_block >= election.start_block, Error::<T>::ElectionNotStarted);
            ensure!(current_block < election.end_block, Error::<T>::ElectionEnded);
            
            // Check voter hasn't already voted
            ensure!(!HasVoted::<T>::contains_key(&voter), Error::<T>::AlreadyVoted);
            
            // Check candidate exists
            let candidates = Candidates::<T>::get();
            ensure!(
                candidates.iter().any(|c| c.id == candidate_id),
                Error::<T>::InvalidCandidate
            );
            
            // Record the vote
            HasVoted::<T>::insert(&voter, candidate_id);
            
            // Increment vote count for candidate
            VoteCount::<T>::mutate(candidate_id, |count| {
                *count = count.saturating_add(1);
            });
            
            // Increment total votes
            TotalVotes::<T>::mutate(|total| {
                *total = total.saturating_add(1);
            });
            
            Self::deposit_event(Event::VoteCast {
                voter,
                candidate_id,
            });
            
            Ok(())
        }

        /// End the election (Admin only)
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000,0))]
        pub fn end_election(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            
            Election::<T>::try_mutate(|election_opt| {
                let election = election_opt.as_mut().ok_or(Error::<T>::NoElectionExists)?;
                
                election.is_active = false;
                
                Self::deposit_event(Event::ElectionEnded);
                Ok(())
            })
        }

        /// Finalize election results (Admin only)
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000,0))]
        pub fn finalize_election(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            
            Election::<T>::try_mutate(|election_opt| {
                let election = election_opt.as_mut().ok_or(Error::<T>::NoElectionExists)?;
                ensure!(!election.is_active, Error::<T>::ElectionIsActive);
                ensure!(!election.is_finalized, Error::<T>::AlreadyFinalized);
                
                election.is_finalized = true;
                
                Self::deposit_event(Event::ElectionFinalized);
                Ok(())
            })
        }

        /// Reset election (Admin only - for testing or new election)
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(10_000,0))]
        pub fn reset_election(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            
            // Clear all storage
            Election::<T>::kill();
            Candidates::<T>::kill();
            let _ = HasVoted::<T>::clear(u32::MAX, None);
            let _ = VoteCount::<T>::clear(u32::MAX, None);
            TotalVotes::<T>::kill();
            
            Self::deposit_event(Event::ElectionReset);
            
            Ok(())
        }
    }

    // ==================== HELPER FUNCTIONS ====================

    impl<T: Config> Pallet<T> {
        /// Get election results
        pub fn get_results() -> Vec<(u32, Vec<u8>, u32)> {
            let candidates = Candidates::<T>::get();
            candidates
                .iter()
                .map(|candidate| {
                    let votes = VoteCount::<T>::get(candidate.id);
                    (candidate.id, candidate.name.to_vec(), votes)
                })
                .collect()
        }

        /// Check if a specific account has voted
        pub fn has_account_voted(account: &T::AccountId) -> bool {
            HasVoted::<T>::contains_key(account)
        }

        /// Get the candidate ID that an account voted for (if any)
        pub fn get_vote_for_account(account: &T::AccountId) -> Option<u32> {
            HasVoted::<T>::get(account)
        }
    }
}