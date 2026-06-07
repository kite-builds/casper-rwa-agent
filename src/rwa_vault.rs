//! RwaVault — an on-chain vault for a tokenized real-world asset (RWA).
//!
//! The vault registers fractional shareholders of a single real-world asset
//! (e.g. a rental property), accepts rent deposits in native CSPR, and
//! distributes the accumulated rent pro-rata to every shareholder in a single
//! on-chain settlement transaction.
//!
//! In the CasperRWA-Agent system this contract is the on-chain settlement
//! layer. An off-chain autonomous agent pays an x402-gated oracle to learn when
//! rent is due, then calls [`RwaVault::deposit_rent`] + [`RwaVault::distribute`]
//! to settle on Casper — no human in the loop.
use odra::casper_types::U512;
use odra::prelude::*;

/// RwaVault contract.
#[odra::module(
    errors = Error,
    events = [ShareholderRegistered, RentDeposited, Distributed]
)]
pub struct RwaVault {
    /// Human-readable description / identifier of the tokenized asset.
    asset: Var<String>,
    /// Contract administrator (set to the deployer at init).
    owner: Var<Address>,
    /// shareholder address -> share units held.
    shares: Mapping<Address, U512>,
    /// Sum of all issued share units (the pro-rata denominator).
    total_shares: Var<U512>,
    /// Dense index of registered shareholders, so `distribute` can enumerate
    /// them (an Odra `Mapping` is not iterable on its own).
    holder_at: Mapping<u32, Address>,
    /// Number of registered shareholders.
    holder_count: Var<u32>,
    /// Rent deposited but not yet distributed (in motes).
    rent_pool: Var<U512>,
    /// Lifetime rent distributed to shareholders (in motes).
    total_distributed: Var<U512>,
}

#[odra::module]
impl RwaVault {
    /// Initializes the vault for one asset. The caller becomes the owner.
    pub fn init(&mut self, asset: String) {
        self.asset.set(asset);
        self.owner.set(self.env().caller());
        self.total_shares.set(U512::zero());
        self.holder_count.set(0);
        self.rent_pool.set(U512::zero());
        self.total_distributed.set(U512::zero());
    }

    /// Registers a fractional shareholder with `share_units` of the asset.
    /// Owner-only. A given address may be registered once.
    pub fn register_shareholder(&mut self, holder: Address, share_units: U512) {
        self.assert_owner();
        if share_units.is_zero() {
            self.env().revert(Error::ZeroShares);
        }
        if !self.shares.get_or_default(&holder).is_zero() {
            self.env().revert(Error::AlreadyRegistered);
        }

        let idx = self.holder_count.get_or_default();
        self.holder_at.set(&idx, holder);
        self.holder_count.set(idx + 1);

        self.shares.set(&holder, share_units);
        self.total_shares.add(share_units);

        self.env().emit_event(ShareholderRegistered {
            holder,
            share_units,
        });
    }

    /// Deposits rent (attached native CSPR) into the undistributed pool.
    /// Permissionless: the autonomous agent (or anyone) may fund rent.
    #[odra(payable)]
    pub fn deposit_rent(&mut self) {
        let amount = self.env().attached_value();
        if amount.is_zero() {
            self.env().revert(Error::NothingToDeposit);
        }
        self.rent_pool.add(amount);
        self.env().emit_event(RentDeposited {
            from: self.env().caller(),
            amount,
        });
    }

    /// Distributes the entire rent pool pro-rata to all shareholders in one
    /// transaction. Permissionless — anyone (typically the agent) may trigger a
    /// settlement once rent is due. Integer-division dust is retained in the
    /// pool for the next round.
    pub fn distribute(&mut self) {
        let pool = self.rent_pool.get_or_default();
        if pool.is_zero() {
            self.env().revert(Error::NothingToDistribute);
        }
        let total = self.total_shares.get_or_default();
        if total.is_zero() {
            self.env().revert(Error::NoShareholders);
        }

        let count = self.holder_count.get_or_default();
        let mut paid_out = U512::zero();
        for idx in 0..count {
            let holder = match self.holder_at.get(&idx) {
                Some(h) => h,
                None => continue,
            };
            let holder_shares = self.shares.get_or_default(&holder);
            if holder_shares.is_zero() {
                continue;
            }
            // pro-rata share of the pool: pool * holder_shares / total
            let amount = pool * holder_shares / total;
            if amount.is_zero() {
                continue;
            }
            self.env().transfer_tokens(&holder, &amount);
            paid_out += amount;
        }

        // Retain integer-division dust for the next distribution round.
        self.rent_pool.subtract(paid_out);
        self.total_distributed.add(paid_out);

        self.env().emit_event(Distributed {
            amount: paid_out,
            shareholders: count,
        });
    }

    // ---- views ----

    /// Returns the asset description.
    pub fn asset(&self) -> String {
        self.asset.get_or_default()
    }

    /// Returns the vault owner.
    pub fn get_owner(&self) -> Address {
        self.owner.get().unwrap_or_revert_with(&self.env(), Error::NotInitialized)
    }

    /// Returns the share units held by `holder`.
    pub fn shares_of(&self, holder: &Address) -> U512 {
        self.shares.get_or_default(holder)
    }

    /// Returns the total issued share units.
    pub fn total_shares(&self) -> U512 {
        self.total_shares.get_or_default()
    }

    /// Returns the number of registered shareholders.
    pub fn shareholder_count(&self) -> u32 {
        self.holder_count.get_or_default()
    }

    /// Returns the undistributed rent pool (motes).
    pub fn rent_pool(&self) -> U512 {
        self.rent_pool.get_or_default()
    }

    /// Returns the lifetime distributed rent (motes).
    pub fn total_distributed(&self) -> U512 {
        self.total_distributed.get_or_default()
    }

    // ---- internal ----

    fn assert_owner(&self) {
        let owner = self
            .owner
            .get()
            .unwrap_or_revert_with(&self.env(), Error::NotInitialized);
        if self.env().caller() != owner {
            self.env().revert(Error::NotOwner);
        }
    }
}

/// Errors that may occur during vault execution.
#[odra::odra_error]
pub enum Error {
    /// A non-owner called an owner-only entrypoint.
    NotOwner = 1,
    /// A shareholder was registered with zero share units.
    ZeroShares = 2,
    /// The address is already a registered shareholder.
    AlreadyRegistered = 3,
    /// `deposit_rent` was called with no attached value.
    NothingToDeposit = 4,
    /// `distribute` was called with an empty rent pool.
    NothingToDistribute = 5,
    /// `distribute` was called before any shareholder was registered.
    NoShareholders = 6,
    /// The owner var was unset (contract not initialized).
    NotInitialized = 7,
}

/// Emitted when a shareholder is registered.
#[odra::event]
pub struct ShareholderRegistered {
    /// The registered shareholder.
    pub holder: Address,
    /// The share units assigned.
    pub share_units: U512,
}

/// Emitted when rent is deposited into the pool.
#[odra::event]
pub struct RentDeposited {
    /// Who funded the rent (typically the autonomous agent).
    pub from: Address,
    /// Amount deposited (motes).
    pub amount: U512,
}

/// Emitted when a distribution settles.
#[odra::event]
pub struct Distributed {
    /// Total amount paid out this round (motes).
    pub amount: U512,
    /// Number of shareholders considered.
    pub shareholders: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::host::{Deployer, HostRef};

    fn setup() -> (RwaVaultHostRef, Address, Address, Address) {
        let env = odra_test::env();
        let vault = RwaVault::deploy(
            &env,
            RwaVaultInitArgs {
                asset: String::from("Oslo duplex, Storgata 1"),
            },
        );
        let owner = env.get_account(0);
        let alice = env.get_account(1);
        let bob = env.get_account(2);
        (vault, owner, alice, bob)
    }

    #[test]
    fn init_sets_owner_and_asset() {
        let (vault, owner, _, _) = setup();
        assert_eq!(vault.get_owner(), owner);
        assert_eq!(vault.asset(), "Oslo duplex, Storgata 1");
        assert!(vault.total_shares().is_zero());
        assert_eq!(vault.shareholder_count(), 0);
    }

    #[test]
    fn register_shareholders_tracks_totals() {
        let (mut vault, _, alice, bob) = setup();
        vault.register_shareholder(alice, U512::from(60));
        vault.register_shareholder(bob, U512::from(40));
        assert_eq!(vault.shares_of(&alice), U512::from(60));
        assert_eq!(vault.shares_of(&bob), U512::from(40));
        assert_eq!(vault.total_shares(), U512::from(100));
        assert_eq!(vault.shareholder_count(), 2);
    }

    #[test]
    fn non_owner_cannot_register() {
        let (mut vault, _, alice, bob) = setup();
        vault.env().set_caller(alice);
        assert_eq!(
            vault
                .try_register_shareholder(bob, U512::from(10))
                .unwrap_err(),
            Error::NotOwner.into()
        );
    }

    #[test]
    fn cannot_register_zero_or_twice() {
        let (mut vault, _, alice, _) = setup();
        assert_eq!(
            vault
                .try_register_shareholder(alice, U512::zero())
                .unwrap_err(),
            Error::ZeroShares.into()
        );
        vault.register_shareholder(alice, U512::from(10));
        assert_eq!(
            vault
                .try_register_shareholder(alice, U512::from(5))
                .unwrap_err(),
            Error::AlreadyRegistered.into()
        );
    }

    #[test]
    fn deposit_rent_accumulates_pool() {
        let (mut vault, _, alice, _) = setup();
        vault.register_shareholder(alice, U512::from(100));
        vault.with_tokens(U512::from(1_000)).deposit_rent();
        assert_eq!(vault.rent_pool(), U512::from(1_000));
    }

    #[test]
    fn distribute_pays_pro_rata() {
        let (mut vault, _, alice, bob) = setup();
        let env = vault.env().clone();
        vault.register_shareholder(alice, U512::from(60));
        vault.register_shareholder(bob, U512::from(40));

        vault.with_tokens(U512::from(1_000)).deposit_rent();

        let alice_before = env.balance_of(&alice);
        let bob_before = env.balance_of(&bob);

        vault.distribute();

        // 60/40 split of 1000 = 600 / 400, no dust.
        assert_eq!(env.balance_of(&alice) - alice_before, U512::from(600));
        assert_eq!(env.balance_of(&bob) - bob_before, U512::from(400));
        assert!(vault.rent_pool().is_zero());
        assert_eq!(vault.total_distributed(), U512::from(1_000));
    }

    #[test]
    fn distribute_retains_dust() {
        let (mut vault, _, alice, bob) = setup();
        vault.register_shareholder(alice, U512::from(1));
        vault.register_shareholder(bob, U512::from(2));
        // 100 / 3 shares: alice 33, bob 66, dust 1 retained.
        vault.with_tokens(U512::from(100)).deposit_rent();
        vault.distribute();
        assert_eq!(vault.total_distributed(), U512::from(99));
        assert_eq!(vault.rent_pool(), U512::from(1));
    }

    #[test]
    fn distribute_empty_pool_reverts() {
        let (mut vault, _, alice, _) = setup();
        vault.register_shareholder(alice, U512::from(10));
        assert_eq!(
            vault.try_distribute().unwrap_err(),
            Error::NothingToDistribute.into()
        );
    }
}
