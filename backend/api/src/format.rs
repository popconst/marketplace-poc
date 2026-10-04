//! Money and counts formatted for the messages the API shows to people.

/// `€90`, or `€1,191.90` when there are cents. Takes non-negative euro cents.
pub fn euros(cents: i64) -> String {
    let (whole, cents) = (grouped(cents / 100), cents % 100);
    if cents == 0 {
        format!("€{whole}")
    } else {
        format!("€{whole}.{cents:02}")
    }
}

/// `3,158`: thousands separated by commas. Takes a non-negative number.
pub fn grouped(n: i64) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_euros_leave_the_cents_out() {
        assert_eq!(euros(9_000), "€90");
        assert_eq!(euros(504_500), "€5,045");
    }

    #[test]
    fn euros_with_cents_show_both_digits() {
        assert_eq!(euros(10_750), "€107.50");
        assert_eq!(euros(32_572), "€325.72");
        assert_eq!(euros(5), "€0.05");
    }

    #[test]
    fn thousands_are_separated_by_commas() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }
}
