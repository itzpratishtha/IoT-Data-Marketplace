#![allow(non_snake_case)]
#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, log, Env, Symbol, String, symbol_short, Address, Vec};

// Define asset status to track leased assets
#[contracttype]
#[derive(Clone)]
pub struct AssetStats {
    pub available: u64,   // Count of assets available for lease
    pub leased: u64,      // Count of currently leased assets
    pub registered: u64,  // Total count of registered assets
    pub total_revenue: u64 // Total revenue generated from leases (in stroop - millionth of XLM)
}

// Reference to the AssetStats struct - fixed symbol length
const ALL_ASSETS: Symbol = symbol_short!("ALL_ASSET");

// Payment models supported by the platform
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum PaymentModel {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    PayPerUse
}

// Define the Asset structure for registration
#[contracttype]
#[derive(Clone)]
pub struct Asset {
    pub asset_id: u64,
    pub owner: Address,
    pub title: String,
    pub description: String,
    pub asset_type: String,  // "Physical" or "Digital"
    pub location: String,    // Geographic location or "Digital"
    pub price: u64,          // Price in stroop (millionth of XLM)
    pub payment_model: PaymentModel,
    pub is_available: bool,
    pub created_time: u64,
    pub quality_guarantee: String,
    pub rating: u64          // Rating out of 100
}

// Mapping asset_id to Asset
#[contracttype] 
pub enum AssetRegistry { 
    Asset(u64)
}

// Reference to the asset counter for unique IDs
const ASSET_COUNTER: Symbol = symbol_short!("ASSET_CNT");

// Define a Lease structure to track active leases
#[contracttype]
#[derive(Clone)]
pub struct Lease {
    pub lease_id: u64,
    pub asset_id: u64,
    pub lessor: Address,
    pub lessee: Address,
    pub start_time: u64,
    pub end_time: u64,
    pub total_cost: u64,
    pub is_active: bool,
    pub is_paid: bool,
    pub access_key: String,  // Encrypted access key for the asset
    pub dispute_raised: bool
}

// Mapping lease_id to Lease
#[contracttype] 
pub enum LeaseRegistry { 
    Lease(u64)
}

// Reference to the lease counter for unique IDs
const LEASE_COUNTER: Symbol = symbol_short!("LEASE_CNT");

// Define a Review structure to track user reviews
#[contracttype]
#[derive(Clone)]
pub struct Review {
    pub review_id: u64,
    pub asset_id: u64,
    pub reviewer: Address,
    pub rating: u64,         // Rating out of 100
    pub comment: String,
    pub review_time: u64
}

// Mapping review_id to Review
#[contracttype] 
pub enum ReviewRegistry { 
    Review(u64)
}

// Reference to the review counter for unique IDs - fixed symbol length
const REVIEW_COUNTER: Symbol = symbol_short!("REV_CNT");

// Main contract definition
#[contract]
pub struct IoTMarketplace;

#[contractimpl]
impl IoTMarketplace {
    
    // Initialize the marketplace
    pub fn initialize(env: Env) {
        // Set initial asset stats
        let asset_stats = AssetStats {
            available: 0,
            leased: 0,
            registered: 0,
            total_revenue: 0
        };
        
        // Store initial stats
        env.storage().instance().set(&ALL_ASSETS, &asset_stats);
        
        // Initialize counters
        env.storage().instance().set(&ASSET_COUNTER, &0u64);
        env.storage().instance().set(&LEASE_COUNTER, &0u64);
        env.storage().instance().set(&REVIEW_COUNTER, &0u64);
        
        // Set contract TTL
        env.storage().instance().extend_ttl(10000, 10000);
        
        log!(&env, "IoT Marketplace initialized");
    }
    
    // Register a new asset
    pub fn register_asset(
        env: Env,
        owner: Address,
        title: String,
        description: String,
        asset_type: String,
        location: String,
        price: u64,
        payment_model: PaymentModel,
        quality_guarantee: String
    ) -> u64 {
        // Verify the caller is the owner
        owner.require_auth();
        
        // Get current asset counter
        let mut asset_counter: u64 = env.storage().instance().get(&ASSET_COUNTER).unwrap_or(0);
        asset_counter += 1;
        
        // Get current timestamp
        let time = env.ledger().timestamp();
        
        // Create new asset
        let asset = Asset {
            asset_id: asset_counter,
            owner: owner.clone(),
            title,
            description,
            asset_type,
            location,
            price,
            payment_model,
            is_available: true,
            created_time: time,
            quality_guarantee,
            rating: 0  // Initial rating
        };
        
        // Update asset stats
        let mut stats = Self::get_asset_stats(env.clone());
        stats.registered += 1;
        stats.available += 1;
        
        // Store the asset
        env.storage().instance().set(&AssetRegistry::Asset(asset_counter), &asset);
        
        // Update counter and stats
        env.storage().instance().set(&ASSET_COUNTER, &asset_counter);
        env.storage().instance().set(&ALL_ASSETS, &stats);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Asset registered with ID: {}", asset_counter);
        return asset_counter;
    }
    
    // Update asset details
    pub fn update_asset(
        env: Env,
        asset_id: u64,
        owner: Address,
        title: String,
        description: String,
        price: u64,
        is_available: bool,
        quality_guarantee: String
    ) -> bool {
        // Verify the caller is the owner
        owner.require_auth();
        
        // Get the asset
        let mut asset = Self::get_asset(env.clone(), asset_id);
        
        // Verify ownership
        if asset.owner != owner {
            log!(&env, "Only the owner can update the asset");
            panic!("Only the owner can update the asset");
        }
        
        // Update asset stats if availability changes
        let mut stats = Self::get_asset_stats(env.clone());
        if asset.is_available != is_available {
            if is_available {
                stats.available += 1;
            } else {
                stats.available -= 1;
            }
            env.storage().instance().set(&ALL_ASSETS, &stats);
        }
        
        // Update asset fields
        asset.title = title;
        asset.description = description;
        asset.price = price;
        asset.is_available = is_available;
        asset.quality_guarantee = quality_guarantee;
        
        // Store updated asset
        env.storage().instance().set(&AssetRegistry::Asset(asset_id), &asset);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Asset updated: {}", asset_id);
        return true;
    }
    
    // Create a new lease
    pub fn create_lease(
        env: Env,
        asset_id: u64,
        lessee: Address,
        duration: u64,  // Duration in seconds
        access_key: String
    ) -> u64 {
        // Verify the caller is the lessee
        lessee.require_auth();
        
        // Get the asset
        let mut asset = Self::get_asset(env.clone(), asset_id);
        
        // Verify asset is available
        if !asset.is_available {
            log!(&env, "Asset is not available for lease");
            panic!("Asset is not available for lease");
        }
        
        // Get current timestamp
        let time = env.ledger().timestamp();
        let end_time = time + duration;
        
        // Calculate total cost based on payment model and duration
        let total_cost = match asset.payment_model {
            PaymentModel::Hourly => asset.price * (duration / 3600),
            PaymentModel::Daily => asset.price * (duration / 86400),
            PaymentModel::Weekly => asset.price * (duration / 604800),
            PaymentModel::Monthly => asset.price * (duration / 2592000),
            PaymentModel::PayPerUse => asset.price
        };
        
        // Get current lease counter
        let mut lease_counter: u64 = env.storage().instance().get(&LEASE_COUNTER).unwrap_or(0);
        lease_counter += 1;
        
        // Create new lease
        let lease = Lease {
            lease_id: lease_counter,
            asset_id,
            lessor: asset.owner.clone(),
            lessee: lessee.clone(),
            start_time: time,
            end_time,
            total_cost,
            is_active: true,
            is_paid: false,  // Will be set to true after payment
            access_key,
            dispute_raised: false
        };
        
        // Update asset availability
        asset.is_available = false;
        env.storage().instance().set(&AssetRegistry::Asset(asset_id), &asset);
        
        // Update asset stats
        let mut stats = Self::get_asset_stats(env.clone());
        stats.available -= 1;
        stats.leased += 1;
        env.storage().instance().set(&ALL_ASSETS, &stats);
        
        // Store the lease
        env.storage().instance().set(&LeaseRegistry::Lease(lease_counter), &lease);
        
        // Update counter
        env.storage().instance().set(&LEASE_COUNTER, &lease_counter);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Lease created with ID: {}", lease_counter);
        return lease_counter;
    }
    
    // Process payment for a lease
    pub fn process_payment(
        env: Env,
        lease_id: u64,
        payer: Address
    ) -> bool {
        // Verify the caller is the payer (lessee)
        payer.require_auth();
        
        // Get the lease
        let mut lease = Self::get_lease(env.clone(), lease_id);
        
        // Verify payer is the lessee
        if lease.lessee != payer {
            log!(&env, "Only the lessee can make the payment");
            panic!("Only the lessee can make the payment");
        }
        
        // Verify lease is active and not already paid
        if !lease.is_active || lease.is_paid {
            log!(&env, "Lease is not active or already paid");
            panic!("Lease is not active or already paid");
        }
        
        // In real implementation, this would involve token transfer
        // For now, we'll just mark it as paid
        
        // Update lease as paid
        lease.is_paid = true;
        env.storage().instance().set(&LeaseRegistry::Lease(lease_id), &lease);
        
        // Update total revenue
        let mut stats = Self::get_asset_stats(env.clone());
        stats.total_revenue += lease.total_cost;
        env.storage().instance().set(&ALL_ASSETS, &stats);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Payment processed for lease: {}", lease_id);
        return true;
    }
    
    // End a lease (early termination or expiration)
    pub fn end_lease(
        env: Env,
        lease_id: u64,
        caller: Address
    ) -> bool {
        // Verify the caller is either the lessor or lessee
        caller.require_auth();
        
        // Get the lease
        let mut lease = Self::get_lease(env.clone(), lease_id);
        
        // Verify caller is either lessor or lessee
        if lease.lessor != caller && lease.lessee != caller {
            log!(&env, "Only the lessor or lessee can end the lease");
            panic!("Only the lessor or lessee can end the lease");
        }
        
        // Verify lease is active
        if !lease.is_active {
            log!(&env, "Lease is not active");
            panic!("Lease is not active");
        }
        
        // Mark lease as inactive
        lease.is_active = false;
        env.storage().instance().set(&LeaseRegistry::Lease(lease_id), &lease);
        
        // Update asset availability
        let mut asset = Self::get_asset(env.clone(), lease.asset_id);
        asset.is_available = true;
        env.storage().instance().set(&AssetRegistry::Asset(lease.asset_id), &asset);
        
        // Update asset stats
        let mut stats = Self::get_asset_stats(env.clone());
        stats.available += 1;
        stats.leased -= 1;
        env.storage().instance().set(&ALL_ASSETS, &stats);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Lease ended: {}", lease_id);
        return true;
    }
    
    // Submit a review for an asset
    pub fn submit_review(
        env: Env,
        asset_id: u64,
        reviewer: Address,
        rating: u64,
        comment: String
    ) -> u64 {
        // Verify the caller is the reviewer
        reviewer.require_auth();
        
        // Verify rating is valid (0-100)
        if rating > 100 {
            log!(&env, "Rating must be between 0 and 100");
            panic!("Rating must be between 0 and 100");
        }
        
        // Get current timestamp
        let time = env.ledger().timestamp();
        
        // Get current review counter
        let mut review_counter: u64 = env.storage().instance().get(&REVIEW_COUNTER).unwrap_or(0);
        review_counter += 1;
        
        // Create new review
        let review = Review {
            review_id: review_counter,
            asset_id,
            reviewer: reviewer.clone(),
            rating,
            comment,
            review_time: time
        };
        
        // Store the review
        env.storage().instance().set(&ReviewRegistry::Review(review_counter), &review);
        
        // Update counter
        env.storage().instance().set(&REVIEW_COUNTER, &review_counter);
        
        // Update asset rating (average of all ratings)
        // In a real implementation, you would calculate the average of all reviews
        // Here, we'll just update directly
        let mut asset = Self::get_asset(env.clone(), asset_id);
        asset.rating = rating;  // Simplified - should be average of all ratings
        env.storage().instance().set(&AssetRegistry::Asset(asset_id), &asset);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Review submitted: {}", review_counter);
        return review_counter;
    }
    
    // Raise a dispute for a lease
    pub fn raise_dispute(
        env: Env,
        lease_id: u64,
        caller: Address
    ) -> bool {
        // Verify the caller is either the lessor or lessee
        caller.require_auth();
        
        // Get the lease
        let mut lease = Self::get_lease(env.clone(), lease_id);
        
        // Verify caller is either lessor or lessee
        if lease.lessor != caller && lease.lessee != caller {
            log!(&env, "Only the lessor or lessee can raise a dispute");
            panic!("Only the lessor or lessee can raise a dispute");
        }
        
        // Verify lease is active
        if !lease.is_active {
            log!(&env, "Cannot raise dispute on inactive lease");
            panic!("Cannot raise dispute on inactive lease");
        }
        
        // Mark dispute as raised
        lease.dispute_raised = true;
        env.storage().instance().set(&LeaseRegistry::Lease(lease_id), &lease);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Dispute raised for lease: {}", lease_id);
        return true;
    }
    
    // Resolve a dispute for a lease
    pub fn resolve_dispute(
        env: Env,
        lease_id: u64,
        admin: Address,
        refund_percentage: u64
    ) -> bool {
        // Verify the caller is an admin (in a real implementation, this would be a multi-sig or DAO)
        admin.require_auth();
        
        // Verify refund percentage is valid (0-100)
        if refund_percentage > 100 {
            log!(&env, "Refund percentage must be between 0 and 100");
            panic!("Refund percentage must be between 0 and 100");
        }
        
        // Get the lease
        let mut lease = Self::get_lease(env.clone(), lease_id);
        
        // Verify dispute was raised
        if !lease.dispute_raised {
            log!(&env, "No dispute raised for this lease");
            panic!("No dispute raised for this lease");
        }
        
        // Calculate refund amount
        let refund_amount = (lease.total_cost * refund_percentage) / 100;
        
        // In real implementation, this would involve token transfers
        // For refunds to the lessee and remaining payment to the lessor
        
        // Mark dispute as resolved and lease as inactive
        lease.dispute_raised = false;
        lease.is_active = false;
        env.storage().instance().set(&LeaseRegistry::Lease(lease_id), &lease);
        
        // Update asset availability
        let mut asset = Self::get_asset(env.clone(), lease.asset_id);
        asset.is_available = true;
        env.storage().instance().set(&AssetRegistry::Asset(lease.asset_id), &asset);
        
        // Update asset stats
        let mut stats = Self::get_asset_stats(env.clone());
        stats.available += 1;
        stats.leased -= 1;
        // Adjust revenue based on refund
        stats.total_revenue -= refund_amount;
        env.storage().instance().set(&ALL_ASSETS, &stats);
        
        // Update contract TTL
        env.storage().instance().extend_ttl(5000, 5000);
        
        log!(&env, "Dispute resolved for lease: {}", lease_id);
        return true;
    }
    
    // Get assets by owner
    pub fn get_assets_by_owner(env: Env, owner: Address) -> Vec<Asset> {
        let asset_counter: u64 = env.storage().instance().get(&ASSET_COUNTER).unwrap_or(0);
        let mut assets = Vec::new(&env);
        
        // Iterate through all assets and find those owned by the specified owner
        for i in 1..=asset_counter {
            let asset = Self::get_asset(env.clone(), i);
            if asset.owner == owner {
                assets.push_back(asset);
            }
        }
        
        return assets;
    }
    
    // Get active leases by lessee
    pub fn get_leases_by_lessee(env: Env, lessee: Address) -> Vec<Lease> {
        let lease_counter: u64 = env.storage().instance().get(&LEASE_COUNTER).unwrap_or(0);
        let mut leases = Vec::new(&env);
        
        // Iterate through all leases and find active ones for the specified lessee
        for i in 1..=lease_counter {
            let lease = Self::get_lease(env.clone(), i);
            if lease.lessee == lessee && lease.is_active {
                leases.push_back(lease);
            }
        }
        
        return leases;
    }
    
    // Get available assets by type
    pub fn get_available_assets_by_type(env: Env, asset_type: String) -> Vec<Asset> {
        let asset_counter: u64 = env.storage().instance().get(&ASSET_COUNTER).unwrap_or(0);
        let mut assets = Vec::new(&env);
        
        // Iterate through all assets and find available ones of the specified type
        for i in 1..=asset_counter {
            let asset = Self::get_asset(env.clone(), i);
            if asset.is_available && asset.asset_type == asset_type {
                assets.push_back(asset);
            }
        }
        
        return assets;
    }
    
    // Helper function to get asset stats
    pub fn get_asset_stats(env: Env) -> AssetStats {
        env.storage().instance().get(&ALL_ASSETS).unwrap_or(AssetStats {
            available: 0,
            leased: 0,
            registered: 0,
            total_revenue: 0
        })
    }
    
    // Helper function to get an asset by ID
    pub fn get_asset(env: Env, asset_id: u64) -> Asset {
        let key = AssetRegistry::Asset(asset_id);
        env.storage().instance().get(&key).unwrap_or_else(|| {
            log!(&env, "Asset not found: {}", asset_id);
            panic!("Asset not found");
        })
    }
    
    // Helper function to get a lease by ID
    pub fn get_lease(env: Env, lease_id: u64) -> Lease {
        let key = LeaseRegistry::Lease(lease_id);
        env.storage().instance().get(&key).unwrap_or_else(|| {
            log!(&env, "Lease not found: {}", lease_id);
            panic!("Lease not found");
        })
    }
    
    // Helper function to get a review by ID
    pub fn get_review(env: Env, review_id: u64) -> Review {
        let key = ReviewRegistry::Review(review_id);
        env.storage().instance().get(&key).unwrap_or_else(|| {
            log!(&env, "Review not found: {}", review_id);
            panic!("Review not found");
        })
    }
}