use crate::error::{AppError, AppResult};
use crate::models::{CartItem, CreateSaleItemRequest};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Shopping cart for POS terminal
#[derive(Debug, Clone)]
pub struct Cart {
    items: HashMap<i32, CartItem>,
}

impl Cart {
    /// Create a new empty cart
    pub fn new() -> Self {
        Cart {
            items: HashMap::new(),
        }
    }

    /// Add item to cart
    pub fn add_item(
        &mut self,
        product_id: i32,
        quantity: i32,
        discount: Option<Decimal>,
    ) -> AppResult<()> {
        if quantity <= 0 {
            return Err(AppError::Validation(
                "Quantity must be greater than 0".to_string(),
            ));
        }

        let cart_item = self.items.entry(product_id).or_insert(CartItem {
            product_id,
            quantity: 0,
            discount: None,
        });

        cart_item.quantity += quantity;
        if let Some(disc) = discount {
            cart_item.discount = Some(disc);
        }

        Ok(())
    }

    /// Update item quantity in cart
    pub fn update_quantity(&mut self, product_id: i32, quantity: i32) -> AppResult<()> {
        if quantity <= 0 {
            return Err(AppError::Validation(
                "Quantity must be greater than 0".to_string(),
            ));
        }

        if let Some(item) = self.items.get_mut(&product_id) {
            item.quantity = quantity;
        } else {
            return Err(AppError::NotFound(format!(
                "Product {} not in cart",
                product_id
            )));
        }

        Ok(())
    }

    /// Remove item from cart
    pub fn remove_item(&mut self, product_id: i32) -> AppResult<()> {
        self.items
            .remove(&product_id)
            .ok_or_else(|| AppError::NotFound(format!("Product {} not in cart", product_id)))?;
        Ok(())
    }

    /// Clear all items from cart
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Get all items in cart
    pub fn items(&self) -> Vec<CartItem> {
        self.items
            .values()
            .map(|item| CartItem {
                product_id: item.product_id,
                quantity: item.quantity,
                discount: item.discount,
            })
            .collect()
    }

    /// Get item count
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Get total quantity of all items
    pub fn total_quantity(&self) -> i32 {
        self.items.values().map(|item| item.quantity).sum()
    }

    /// Check if cart is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Convert cart to sale item requests
    pub fn to_sale_items(&self) -> Vec<CreateSaleItemRequest> {
        self.items
            .values()
            .map(|item| CreateSaleItemRequest {
                product_id: item.product_id,
                quantity: item.quantity,
                discount: item.discount,
                selected_modifiers: None,
            })
            .collect()
    }
}

impl Default for Cart {
    fn default() -> Self {
        Self::new()
    }
}
