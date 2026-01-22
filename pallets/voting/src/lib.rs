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
    
    // Structure for the encrypted vote
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct EncryptedVote<T: Config> {
        pub encrypted_vote: BoundedVec<u8, T::MaxEncryptedVoteSize>,  // Changed :: to :
        pub blind_signature: BoundedVec<u8, T::MaxBlindSignatureSize>,  // Changed :: to :
    }
    
    // Structure for the pending votes
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct PendingVote<T: Config> {  // Changed name from PendingVotes to PendingVote
        pub encrypted_vote: BoundedVec<u8, T::MaxEncryptedVoteSize>,  // Changed :: to :
        pub blind_signature: BoundedVec<u8, T::MaxBlindSignatureSize>,  // Changed :: to :
    }
    
    // Rules for the pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        #[pallet::constant]
        type MaxEncryptedVoteSize: Get<u32>;
        
        #[pallet::constant]
        type MaxBlindSignatureSize: Get<u32>;
    }
    
    // Storage for the verified votes
    #[pallet::storage]
    #[pallet::getter(fn encrypted_votes)]
    pub type EncryptedVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        EncryptedVote<T>,
        OptionQuery,
    >;
    
    #[pallet::storage]
    #[pallet::getter(fn vote_counter)]
    pub type VoteCounter<T: Config> = StorageValue<_, u32, ValueQuery>;
    
    // Storage for the pending votes
    #[pallet::storage]
    #[pallet::getter(fn pending_vote_counter)]
    pub type PendingVoteCounter<T: Config> = StorageValue<_, u32, ValueQuery>;
    
    #[pallet::storage]
    #[pallet::getter(fn pending_votes)]
    pub type PendingVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        PendingVote<T>,  // Changed to PendingVote (singular)
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        VoteSubmittedForApproval {
            pending_id: u32,
        },
        VoteApproved {
            pending_id: u32,
            vote_id: u32,
        },  // Added missing comma
        VoteRejected {
            pending_id: u32,
        },
        VoteVerified {
            vote_id: u32,
            verified_by: T::AccountId,
            is_valid: bool,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        EncryptedVoteTooLarge,
        BlindSignatureTooLarge,
        VoteNotFound,
        NotAnAuthority,
    }
    
    // The callable functions of the pallet which will be called using the API
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(0)]
        pub fn submit_vote(
            origin: OriginFor<T>,
            encrypted_vote: Vec<u8>,
            blind_signature: Vec<u8>,
        ) -> DispatchResult {
            let _voter = ensure_signed(origin)?;

            let bounded_vote = BoundedVec::try_from(encrypted_vote)
                .map_err(|_| Error::<T>::EncryptedVoteTooLarge)?;
            let bounded_signature = BoundedVec::try_from(blind_signature)
                .map_err(|_| Error::<T>::BlindSignatureTooLarge)?;
            
            let pending_vote = PendingVote::<T> {  // Changed to PendingVote
                encrypted_vote: bounded_vote,
                blind_signature: bounded_signature,
            };  // Added semicolon
            
            let pending_id = PendingVoteCounter::<T>::get();  // Changed vote_id to pending_id
            PendingVotes::<T>::insert(pending_id, pending_vote);
            PendingVoteCounter::<T>::put(pending_id.saturating_add(1));

            Self::deposit_event(Event::VoteSubmittedForApproval {
                pending_id,  // Now this variable exists
            });
            
            Ok(())
        }
        
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(1)]
        pub fn approve_vote(
            origin: OriginFor<T>,  // Changed period to comma
            pending_id: u32,
        ) -> DispatchResult {
            let _authority = ensure_signed(origin)?;

            let pending_vote = PendingVotes::<T>::get(pending_id)  // Changed period to nothing
                .ok_or(Error::<T>::VoteNotFound)?;
            
            let approved_vote = EncryptedVote::<T> {
                encrypted_vote: pending_vote.encrypted_vote,  // Fixed typo
                blind_signature: pending_vote.blind_signature,
            };
            
            let vote_id = VoteCounter::<T>::get();
            EncryptedVotes::<T>::insert(vote_id, approved_vote);
            VoteCounter::<T>::put(vote_id.saturating_add(1));
            
            PendingVotes::<T>::remove(pending_id);
            
            Self::deposit_event(Event::VoteApproved {
                pending_id,
                vote_id,
            });
            
            Ok(())
        }
        
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        #[pallet::call_index(2)]
        pub fn reject_vote(
            origin: OriginFor<T>,
            pending_id: u32,
        ) -> DispatchResult {
            let _authority = ensure_signed(origin)?;

            PendingVotes::<T>::remove(pending_id);
            
            Self::deposit_event(Event::VoteRejected {
                pending_id,
            });
            
            Ok(())
        }
    }
}