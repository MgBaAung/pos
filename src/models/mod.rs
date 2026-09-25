pub mod branch;
pub mod category;
pub mod customer;
pub mod inventory_log;
pub mod product;
pub mod product_variant;
pub mod sale;
pub mod user;

// Branch models
pub use branch::{
    AssignUserToBranchRequest, Branch, BranchComparison, BranchInventorySummary, BranchPerformance,
    BranchResponse, CreateBranchRequest, UpdateBranchRequest, UserBranchAccess,
    UserBranchAssignment,
};

// Category models
pub use category::{Category, CreateCategoryRequest, UpdateCategoryRequest};

// Customer models
pub use customer::{CreateCustomerRequest, Customer, CustomerResponse, UpdateCustomerRequest};

// Inventory models
pub use inventory_log::{AdjustmentType, InventoryLog};

// Product models
pub use product::{
    CreateProductRequest, Product, ProductResponse, StockAdjustmentRequest, UpdateProductRequest,
};

// Product variant models
pub use product_variant::{
    CreateProductVariantRequest, ProductVariant, ProductVariantResponse,
    UpdateProductVariantRequest,
};

// Sale models
pub use sale::{
    CartItem, CashDrawerSession, CloseCashDrawerSessionRequest, CreateCashDrawerSessionRequest,
    CreateSaleItemRequest, CreateSaleRequest, PaymentMethod, ReceiptData, ReceiptItem, Sale,
    SaleItem, SaleResponse, SaleStatus,
};

// User models
pub use user::{
    AuthResponse, CreateUserRequest, LoginRequest, UpdateUserRequest, User, UserResponse, UserRole,
};
