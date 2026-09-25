//! ESC/POS receipt rendering.
//!
//! Converts a [`ReceiptData`] into the byte stream understood by generic
//! thermal receipt printers. Hardware transport (USB/serial/network) is out of
//! scope — callers receive raw bytes they can forward to a printer driver.

use crate::models::ReceiptData;

/// Printable columns for a standard 80mm thermal printer (font A).
const LINE_WIDTH: usize = 42;

/// ESC @ — reset the printer to its default state.
const ESC_INIT: [u8; 2] = [0x1B, 0x40];
/// ESC a n — select justification (0 = left, 1 = center).
const ESC_ALIGN_LEFT: [u8; 3] = [0x1B, 0x61, 0x00];
const ESC_ALIGN_CENTER: [u8; 3] = [0x1B, 0x61, 0x01];
/// ESC E n — emphasized (bold) on/off.
const ESC_BOLD_ON: [u8; 3] = [0x1B, 0x45, 0x01];
const ESC_BOLD_OFF: [u8; 3] = [0x1B, 0x45, 0x00];
/// GS V B 0 — feed and perform a partial paper cut.
const GS_PARTIAL_CUT: [u8; 4] = [0x1D, 0x56, 0x42, 0x00];

/// Truncate a string to at most `width` characters.
fn fit(s: &str, width: usize) -> String {
    if s.chars().count() > width {
        s.chars().take(width).collect()
    } else {
        s.to_string()
    }
}

fn push_line(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(text.as_bytes());
    out.push(b'\n');
}

fn push_amount_row(out: &mut Vec<u8>, label: &str, amount: rust_decimal::Decimal) {
    let left = fit(label, LINE_WIDTH - 14);
    push_line(
        out,
        &format!("{:<width$}{:>14.2}", left, amount, width = LINE_WIDTH - 14),
    );
}

/// Render a receipt into ESC/POS bytes ready to send to a thermal printer.
pub fn render_escpos(receipt: &ReceiptData) -> Vec<u8> {
    let mut out = Vec::new();

    out.extend_from_slice(&ESC_INIT);

    // Header
    out.extend_from_slice(&ESC_ALIGN_CENTER);
    out.extend_from_slice(&ESC_BOLD_ON);
    push_line(&mut out, "SALES RECEIPT");
    out.extend_from_slice(&ESC_BOLD_OFF);
    out.extend_from_slice(&ESC_ALIGN_LEFT);

    // Meta
    push_line(&mut out, &format!("Invoice: {}", receipt.invoice_no));
    push_line(&mut out, &format!("Cashier: {}", receipt.cashier_name));
    if let Some(customer) = &receipt.customer_name {
        push_line(&mut out, &format!("Customer: {}", customer));
    }
    push_line(
        &mut out,
        &format!("Date: {}", receipt.created_at.format("%Y-%m-%d %H:%M")),
    );
    push_line(&mut out, &format!("Payment: {}", receipt.payment_method));
    out.push(b'\n');

    // Line items
    push_line(
        &mut out,
        &format!("{:<24}{:>6}{:>12}", "Item", "Qty", "Amount"),
    );
    push_line(&mut out, &"-".repeat(LINE_WIDTH));
    for item in &receipt.items {
        push_line(
            &mut out,
            &format!(
                "{:<24}{:>6}{:>12.2}",
                fit(&item.name, 24),
                item.quantity,
                item.subtotal
            ),
        );
    }
    out.push(b'\n');

    // Totals
    push_amount_row(&mut out, "Subtotal", receipt.subtotal);
    push_amount_row(&mut out, "Tax", receipt.tax_total);
    push_amount_row(&mut out, "Discount", receipt.discount_total);
    push_line(&mut out, &"-".repeat(LINE_WIDTH));

    out.extend_from_slice(&ESC_BOLD_ON);
    push_amount_row(&mut out, "TOTAL", receipt.grand_total);
    out.extend_from_slice(&ESC_BOLD_OFF);

    push_amount_row(&mut out, "Paid", receipt.amount_paid);
    push_amount_row(&mut out, "Change", receipt.change_given);
    out.push(b'\n');

    // Footer
    out.extend_from_slice(&ESC_ALIGN_CENTER);
    push_line(&mut out, "Thank you!");
    out.extend_from_slice(&ESC_ALIGN_LEFT);

    // Feed so the text clears the tear bar, then cut.
    out.extend_from_slice(b"\n\n\n");
    out.extend_from_slice(&GS_PARTIAL_CUT);

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ReceiptData, ReceiptItem};
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn receipt() -> ReceiptData {
        ReceiptData {
            invoice_no: "INV-0001".into(),
            customer_name: Some("Alice".into()),
            cashier_name: "Bob".into(),
            items: vec![
                ReceiptItem {
                    name: "Coffee".into(),
                    quantity: 2,
                    unit_price: dec!(3.50),
                    discount: dec!(0),
                    subtotal: dec!(7.00),
                },
                ReceiptItem {
                    name: "A very long product name that exceeds the column".into(),
                    quantity: 1,
                    unit_price: dec!(5.25),
                    discount: dec!(0.25),
                    subtotal: dec!(5.00),
                },
            ],
            subtotal: dec!(12.00),
            tax_total: dec!(0.60),
            discount_total: dec!(0.25),
            grand_total: dec!(12.35),
            amount_paid: dec!(20.00),
            change_given: dec!(7.65),
            payment_method: "Cash".into(),
            created_at: Utc::now(),
        }
    }

    fn contains(bytes: &[u8], needle: &str) -> bool {
        bytes.windows(needle.len()).any(|w| w == needle.as_bytes())
    }

    #[test]
    fn starts_with_init_and_ends_with_cut() {
        let out = render_escpos(&receipt());
        assert_eq!(&out[..2], &ESC_INIT);
        assert_eq!(&out[out.len() - 4..], &GS_PARTIAL_CUT);
    }

    #[test]
    fn includes_invoice_and_customer() {
        let out = render_escpos(&receipt());
        assert!(contains(&out, "Invoice: INV-0001"));
        assert!(contains(&out, "Customer: Alice"));
        assert!(contains(&out, "Payment: Cash"));
    }

    #[test]
    fn formats_totals_with_two_decimals() {
        let out = render_escpos(&receipt());
        assert!(contains(&out, "12.35"));
        assert!(contains(&out, "7.65"));
    }

    #[test]
    fn truncates_long_item_names() {
        let out = render_escpos(&receipt());
        let text = String::from_utf8_lossy(&out);
        // The over-long name must be cut to the 24-char item column.
        assert!(text.contains("A very long product name"));
        assert!(!text.contains("exceeds the column"));
    }

    #[test]
    fn no_line_exceeds_width() {
        let out = render_escpos(&receipt());

        // Decode the byte stream, dropping ESC (3-byte) and GS (4-byte) control
        // sequences so only visible glyphs remain, split into printed lines.
        let mut lines: Vec<String> = vec![String::new()];
        let mut i = 0;
        while i < out.len() {
            match out[i] {
                0x1B => i += 3, // ESC x n
                0x1D => i += 4, // GS V B n
                b'\n' => {
                    lines.push(String::new());
                    i += 1;
                }
                b => {
                    lines.last_mut().unwrap().push(b as char);
                    i += 1;
                }
            }
        }

        for line in &lines {
            assert!(
                line.chars().count() <= LINE_WIDTH,
                "line too wide ({}): {:?}",
                line.chars().count(),
                line
            );
        }
    }
}
