use rust_decimal::Decimal;
use crate::api::DiscountType;
use crate::models::PromoCode;

pub fn calculate_discount(
    promo: &PromoCode,
    total_amount: Decimal,
) -> Decimal {
    match promo.discount_type {
        DiscountType::Percentage => {
            let discount = total_amount * promo.discount_value / Decimal::from(100);
            let max_discount = total_amount * Decimal::from(70) / Decimal::from(100);
            if discount > max_discount {
                max_discount
            } else {
                discount
            }
        }
        DiscountType::FixedAmount => {
            if promo.discount_value > total_amount {
                total_amount
            } else {
                promo.discount_value
            }
        }
    }
}
