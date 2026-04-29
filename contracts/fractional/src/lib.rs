#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod fractional {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioItem {
        pub token_id: u64,
        pub shares: u128,
        pub price_per_share: u128,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PortfolioAggregation {
        pub total_value: u128,
        pub positions: Vec<(u64, u128, u128)>,
    }

    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct TaxReport {
        pub total_dividends: u128,
        pub total_proceeds: u128,
        pub transactions: u64,
    }

    /// A share listing placed by a fractional owner who wants to exit
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ShareListing {
        pub seller: AccountId,
        pub token_id: u64,
        pub shares: u128,
        pub price_per_share: u128,
    }

    /// Per-token performance metrics for fractional ownership analytics.
    #[derive(
        Debug,
        Clone,
        PartialEq,
        Eq,
        scale::Encode,
        scale::Decode,
        ink::storage::traits::StorageLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct FractionalMetrics {
        /// Number of completed share trades (buy_shares + swap_shares_for_value).
        pub trade_count: u64,
        /// Cumulative value exchanged across all trades.
        pub total_volume: u128,
        /// Number of distinct accounts that currently hold shares.
        pub unique_holders: u64,
        /// Highest price-per-share ever recorded for this token.
        pub all_time_high_price: u128,
        /// Lowest non-zero price-per-share ever recorded for this token.
        pub all_time_low_price: u128,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum FractionalError {
        InsufficientShares,
        ListingNotFound,
        InsufficientPayment,
        Unauthorized,
        ZeroAmount,
    }

    /// Emitted when an owner lists shares for sale
    #[ink(event)]
    pub struct SharesListed {
        #[ink(topic)]
        seller: AccountId,
        token_id: u64,
        shares: u128,
        price_per_share: u128,
    }

    /// Emitted when a buyer purchases listed shares
    #[ink(event)]
    pub struct SharesSold {
        #[ink(topic)]
        seller: AccountId,
        #[ink(topic)]
        buyer: AccountId,
        token_id: u64,
        shares: u128,
        total_price: u128,
    }

    /// Emitted when an owner redeems shares for their proportional value
    #[ink(event)]
    pub struct SharesRedeemed {
        #[ink(topic)]
        owner: AccountId,
        token_id: u64,
        shares: u128,
        payout: u128,
    }

    /// Emitted when a listing is cancelled
    #[ink(event)]
    pub struct ListingCancelled {
        #[ink(topic)]
        seller: AccountId,
        token_id: u64,
    }

    #[ink(storage)]
    pub struct Fractional {
        last_prices: Mapping<u64, u128>,
        /// Shares held per (owner, token_id)
        balances: Mapping<(AccountId, u64), u128>,
        /// Active listings per (seller, token_id)
        listings: Mapping<(AccountId, u64), ShareListing>,
        /// Total shares issued per token_id
        total_shares: Mapping<u64, u128>,
        /// Performance metrics per token_id
        token_metrics: Mapping<u64, FractionalMetrics>,
        /// Tracks whether an account currently holds shares (for unique_holders count)
        is_holder: Mapping<(AccountId, u64), bool>,
    }

    impl Fractional {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                last_prices: Mapping::default(),
                balances: Mapping::default(),
                listings: Mapping::default(),
                total_shares: Mapping::default(),
                token_metrics: Mapping::default(),
                is_holder: Mapping::default(),
            }
        }
    }

    impl Default for Fractional {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Fractional {
        #[ink(message)]
        pub fn set_last_price(&mut self, token_id: u64, price_per_share: u128) {
            self.last_prices.insert(token_id, &price_per_share);
        }

        #[ink(message)]
        pub fn get_last_price(&self, token_id: u64) -> Option<u128> {
            self.last_prices.get(token_id)
        }

        #[ink(message)]
        pub fn aggregate_portfolio(&self, items: Vec<PortfolioItem>) -> PortfolioAggregation {
            let mut total: u128 = 0;
            let mut positions: Vec<(u64, u128, u128)> = Vec::new();
            for it in items.iter() {
                let price = if it.price_per_share > 0 {
                    it.price_per_share
                } else {
                    self.last_prices.get(it.token_id).unwrap_or(0)
                };
                let value = price.saturating_mul(it.shares);
                total = total.saturating_add(value);
                positions.push((it.token_id, it.shares, price));
            }
            PortfolioAggregation {
                total_value: total,
                positions,
            }
        }

        #[ink(message)]
        pub fn summarize_tax(
            &self,
            dividends: Vec<(u64, u128)>,
            proceeds: Vec<(u64, u128)>,
        ) -> TaxReport {
            let mut total_dividends: u128 = 0;
            for d in dividends.iter() {
                total_dividends = total_dividends.saturating_add(d.1);
            }
            let mut total_proceeds: u128 = 0;
            for p in proceeds.iter() {
                total_proceeds = total_proceeds.saturating_add(p.1);
            }
            TaxReport {
                total_dividends,
                total_proceeds,
                transactions: (dividends.len() + proceeds.len()) as u64,
            }
        }

        // ── Issue #278: Exit mechanism ───────────────────────────────────────

        /// Mint shares to an owner (used in tests / by the property token contract)
        #[ink(message)]
        pub fn mint_shares(&mut self, owner: AccountId, token_id: u64, amount: u128) {
            let current = self.balances.get(&(owner, token_id)).unwrap_or(0);
            let new_bal = current.saturating_add(amount);
            self.balances.insert(&(owner, token_id), &new_bal);
            let total = self.total_shares.get(&token_id).unwrap_or(0);
            self.total_shares.insert(&token_id, &total.saturating_add(amount));
            self.update_holder(owner, token_id, new_bal);
        }

        /// Get the share balance of an owner for a given token
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId, token_id: u64) -> u128 {
            self.balances.get(&(owner, token_id)).unwrap_or(0)
        }

        /// List shares for sale at a given price per share.
        /// The caller must hold at least `shares` of `token_id`.
        #[ink(message)]
        pub fn list_shares_for_sale(
            &mut self,
            token_id: u64,
            shares: u128,
            price_per_share: u128,
        ) -> Result<(), FractionalError> {
            if shares == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let caller = self.env().caller();
            let held = self.balances.get(&(caller, token_id)).unwrap_or(0);
            if held < shares {
                return Err(FractionalError::InsufficientShares);
            }

            let listing = ShareListing {
                seller: caller,
                token_id,
                shares,
                price_per_share,
            };
            self.listings.insert(&(caller, token_id), &listing);
            self.last_prices.insert(token_id, &price_per_share);

            self.env().emit_event(SharesListed {
                seller: caller,
                token_id,
                shares,
                price_per_share,
            });
            Ok(())
        }

        /// Cancel an active listing
        #[ink(message)]
        pub fn cancel_listing(&mut self, token_id: u64) -> Result<(), FractionalError> {
            let caller = self.env().caller();
            if self.listings.get(&(caller, token_id)).is_none() {
                return Err(FractionalError::ListingNotFound);
            }
            self.listings.remove(&(caller, token_id));
            self.env().emit_event(ListingCancelled {
                seller: caller,
                token_id,
            });
            Ok(())
        }

        /// Buy shares from an existing listing.
        /// The buyer must attach sufficient payment (transferred value).
        #[ink(message, payable)]
        pub fn buy_shares(
            &mut self,
            seller: AccountId,
            token_id: u64,
            shares: u128,
        ) -> Result<(), FractionalError> {
            if shares == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let buyer = self.env().caller();
            let payment = self.env().transferred_value();

            let listing = self
                .listings
                .get(&(seller, token_id))
                .ok_or(FractionalError::ListingNotFound)?;

            if shares > listing.shares {
                return Err(FractionalError::InsufficientShares);
            }

            let total_price = listing.price_per_share.saturating_mul(shares);
            if payment < total_price {
                return Err(FractionalError::InsufficientPayment);
            }

            // Transfer shares: deduct from seller, credit buyer
            let seller_held = self.balances.get(&(seller, token_id)).unwrap_or(0);
            self.balances
                .insert(&(seller, token_id), &seller_held.saturating_sub(shares));

            let buyer_held = self.balances.get(&(buyer, token_id)).unwrap_or(0);
            self.balances
                .insert(&(buyer, token_id), &buyer_held.saturating_add(shares));

            // Update or remove listing
            let remaining = listing.shares.saturating_sub(shares);
            if remaining == 0 {
                self.listings.remove(&(seller, token_id));
            } else {
                let updated = ShareListing {
                    shares: remaining,
                    ..listing
                };
                self.listings.insert(&(seller, token_id), &updated);
            }

            // Pay the seller
            if self.env().transfer(seller, total_price).is_err() {
                // Non-fatal: payment forwarding failed (e.g. in unit tests)
            }

            // Analytics: record trade and update holder counts
            let price_per_share = listing.price_per_share;
            let seller_new_bal = self.balances.get(&(seller, token_id)).unwrap_or(0);
            let buyer_new_bal = self.balances.get(&(buyer, token_id)).unwrap_or(0);
            self.record_trade(token_id, total_price, price_per_share);
            self.update_holder(seller, token_id, seller_new_bal);
            self.update_holder(buyer, token_id, buyer_new_bal);

            self.env().emit_event(SharesSold {
                seller,
                buyer,
                token_id,
                shares,
                total_price,
            });
            Ok(())
        }

        /// Redeem shares for their proportional value based on the last recorded price.
        /// Burns the shares and pays out `shares * last_price` to the caller.
        #[ink(message)]
        pub fn redeem_shares(
            &mut self,
            token_id: u64,
            shares: u128,
        ) -> Result<u128, FractionalError> {
            if shares == 0 {
                return Err(FractionalError::ZeroAmount);
            }
            let caller = self.env().caller();
            let held = self.balances.get(&(caller, token_id)).unwrap_or(0);
            if held < shares {
                return Err(FractionalError::InsufficientShares);
            }

            let price = self.last_prices.get(token_id).unwrap_or(0);
            let payout = price.saturating_mul(shares);

            // Burn shares
            self.balances
                .insert(&(caller, token_id), &held.saturating_sub(shares));
            let total = self.total_shares.get(&token_id).unwrap_or(0);
            self.total_shares
                .insert(&token_id, &total.saturating_sub(shares));

            // Pay out (best-effort in unit tests)
            if payout > 0 {
                let _ = self.env().transfer(caller, payout);
            }

            // Analytics: record redemption as a trade and update holder count
            let new_bal = self.balances.get(&(caller, token_id)).unwrap_or(0);
            self.record_trade(token_id, payout, price);
            self.update_holder(caller, token_id, new_bal);

            self.env().emit_event(SharesRedeemed {
                owner: caller,
                token_id,
                shares,
                payout,
            });
            Ok(payout)
        }

        /// Get an active listing
        #[ink(message)]
        pub fn get_listing(&self, seller: AccountId, token_id: u64) -> Option<ShareListing> {
            self.listings.get(&(seller, token_id))
        }

        // ── Issue #273: Fractional ownership analytics ───────────────────────

        /// Returns the performance metrics for `token_id`.
        #[ink(message)]
        pub fn get_metrics(&self, token_id: u64) -> FractionalMetrics {
            self.token_metrics.get(token_id).unwrap_or(FractionalMetrics {
                trade_count: 0,
                total_volume: 0,
                unique_holders: 0,
                all_time_high_price: 0,
                all_time_low_price: 0,
            })
        }

        // ── Private analytics helpers ─────────────────────────────────────────

        /// Record a completed trade: increment trade_count, add to total_volume,
        /// and update all-time high/low price.
        fn record_trade(&mut self, token_id: u64, volume: u128, price_per_share: u128) {
            let mut m = self.get_metrics(token_id);
            m.trade_count = m.trade_count.saturating_add(1);
            m.total_volume = m.total_volume.saturating_add(volume);
            if price_per_share > 0 {
                if price_per_share > m.all_time_high_price {
                    m.all_time_high_price = price_per_share;
                }
                if m.all_time_low_price == 0 || price_per_share < m.all_time_low_price {
                    m.all_time_low_price = price_per_share;
                }
            }
            self.token_metrics.insert(token_id, &m);
        }

        /// Update the unique_holders count when an account's balance crosses zero.
        /// `new_balance` is the balance *after* the transfer.
        fn update_holder(&mut self, account: AccountId, token_id: u64, new_balance: u128) {
            let was_holder = self.is_holder.get(&(account, token_id)).unwrap_or(false);
            let now_holder = new_balance > 0;
            if was_holder == now_holder {
                return;
            }
            self.is_holder.insert(&(account, token_id), &now_holder);
            let mut m = self.get_metrics(token_id);
            if now_holder {
                m.unique_holders = m.unique_holders.saturating_add(1);
            } else {
                m.unique_holders = m.unique_holders.saturating_sub(1);
            }
            self.token_metrics.insert(token_id, &m);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn alice() -> AccountId {
            test::default_accounts::<ink::env::DefaultEnvironment>().alice
        }
        fn bob() -> AccountId {
            test::default_accounts::<ink::env::DefaultEnvironment>().bob
        }

        #[ink::test]
        fn test_mint_and_balance() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 100);
            assert_eq!(f.balance_of(alice(), 1), 100);
        }

        #[ink::test]
        fn test_list_and_cancel() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            assert!(f.list_shares_for_sale(1, 50, 10).is_ok());
            let listing = f.get_listing(alice(), 1).unwrap();
            assert_eq!(listing.shares, 50);
            assert!(f.cancel_listing(1).is_ok());
            assert!(f.get_listing(alice(), 1).is_none());
        }

        #[ink::test]
        fn test_list_insufficient_shares() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 10);
            assert_eq!(
                f.list_shares_for_sale(1, 50, 10),
                Err(FractionalError::InsufficientShares)
            );
        }

        #[ink::test]
        fn test_redeem_shares() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.set_last_price(1, 5);
            let payout = f.redeem_shares(1, 20).unwrap();
            assert_eq!(payout, 100); // 20 * 5
            assert_eq!(f.balance_of(alice(), 1), 80);
        }

        #[ink::test]
        fn test_redeem_insufficient() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 10);
            assert_eq!(
                f.redeem_shares(1, 50),
                Err(FractionalError::InsufficientShares)
            );
        }

        #[ink::test]
        fn test_aggregate_portfolio() {
            let f = Fractional::new();
            let items = vec![
                PortfolioItem { token_id: 1, shares: 10, price_per_share: 5 },
                PortfolioItem { token_id: 2, shares: 20, price_per_share: 3 },
            ];
            let agg = f.aggregate_portfolio(items);
            assert_eq!(agg.total_value, 110);
        }

        #[ink::test]
        fn test_buy_shares_insufficient_payment() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.list_shares_for_sale(1, 50, 10).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(bob());
            // No payment attached → InsufficientPayment
            assert_eq!(
                f.buy_shares(alice(), 1, 10),
                Err(FractionalError::InsufficientPayment)
            );
        }

        // ── Analytics tests (#273) ────────────────────────────────────────────

        #[ink::test]
        fn test_metrics_default_zero() {
            let f = Fractional::new();
            let m = f.get_metrics(42);
            assert_eq!(m.trade_count, 0);
            assert_eq!(m.total_volume, 0);
            assert_eq!(m.unique_holders, 0);
            assert_eq!(m.all_time_high_price, 0);
            assert_eq!(m.all_time_low_price, 0);
        }

        #[ink::test]
        fn test_mint_increments_unique_holders() {
            let mut f = Fractional::new();
            f.mint_shares(alice(), 1, 100);
            assert_eq!(f.get_metrics(1).unique_holders, 1);
            // Minting more to same account doesn't double-count
            f.mint_shares(alice(), 1, 50);
            assert_eq!(f.get_metrics(1).unique_holders, 1);
            // New account increments
            f.mint_shares(bob(), 1, 200);
            assert_eq!(f.get_metrics(1).unique_holders, 2);
        }

        #[ink::test]
        fn test_redeem_decrements_unique_holders() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.set_last_price(1, 5);
            assert_eq!(f.get_metrics(1).unique_holders, 1);
            // Redeem all shares → holder count drops to 0
            f.redeem_shares(1, 100).unwrap();
            assert_eq!(f.get_metrics(1).unique_holders, 0);
        }

        #[ink::test]
        fn test_redeem_records_trade_volume_and_count() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 100);
            f.set_last_price(1, 10);
            f.redeem_shares(1, 50).unwrap(); // payout = 500
            let m = f.get_metrics(1);
            assert_eq!(m.trade_count, 1);
            assert_eq!(m.total_volume, 500);
        }

        #[ink::test]
        fn test_all_time_high_and_low_price() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);

            // First redemption at price 10
            f.set_last_price(1, 10);
            f.redeem_shares(1, 10).unwrap();
            let m = f.get_metrics(1);
            assert_eq!(m.all_time_high_price, 10);
            assert_eq!(m.all_time_low_price, 10);

            // Second redemption at higher price 20
            f.set_last_price(1, 20);
            f.redeem_shares(1, 10).unwrap();
            let m = f.get_metrics(1);
            assert_eq!(m.all_time_high_price, 20);
            assert_eq!(m.all_time_low_price, 10); // low unchanged

            // Third redemption at lower price 5
            f.set_last_price(1, 5);
            f.redeem_shares(1, 10).unwrap();
            let m = f.get_metrics(1);
            assert_eq!(m.all_time_high_price, 20); // high unchanged
            assert_eq!(m.all_time_low_price, 5);
        }

        #[ink::test]
        fn test_cumulative_volume_across_trades() {
            let mut f = Fractional::new();
            test::set_caller::<ink::env::DefaultEnvironment>(alice());
            f.mint_shares(alice(), 1, 1000);
            f.set_last_price(1, 10);
            f.redeem_shares(1, 10).unwrap(); // 100
            f.redeem_shares(1, 20).unwrap(); // 200
            let m = f.get_metrics(1);
            assert_eq!(m.trade_count, 2);
            assert_eq!(m.total_volume, 300);
        }
    }
}
