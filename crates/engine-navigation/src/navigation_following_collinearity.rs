//! Exact collinearity for finite binary32 coordinates.
//!
//! Separately promoted coordinate products are exact in binary64: their
//! significands need at most 48 bits and their exponents stay well inside the
//! binary64 range. The six-term determinant expansion also stays below 2^259.
//! Every product and residual is on the 2^-298 lattice, so grow-expansion
//! arithmetic cannot underflow. These bounds make general TwoSum exact here.

use engine_math::Vector;

const DETERMINANT_TERM_COUNT: usize = 6;

pub(super) fn is_exactly_collinear(position: Vector, start: Vector, end: Vector) -> bool {
    let [px, py, pz] = position.map(f64::from);
    let [ax, ay, az] = start.map(f64::from);
    let [bx, by, bz] = end.map(f64::from);

    let x_terms = [
        py * bz,
        -(py * az),
        -(ay * bz),
        -(pz * by),
        pz * ay,
        az * by,
    ];
    let y_terms = [
        pz * bx,
        -(pz * ax),
        -(az * bx),
        -(px * bz),
        px * az,
        ax * bz,
    ];
    let z_terms = [
        px * by,
        -(px * ay),
        -(ax * by),
        -(py * bx),
        py * ax,
        ay * bx,
    ];

    determinant_is_zero(x_terms) && determinant_is_zero(y_terms) && determinant_is_zero(z_terms)
}

fn determinant_is_zero(terms: [f64; DETERMINANT_TERM_COUNT]) -> bool {
    let mut expansion = [0.0; DETERMINANT_TERM_COUNT];
    let mut length = 0;

    for term in terms {
        length = grow_expansion(&mut expansion, length, term);
    }

    length == 0
}

fn grow_expansion(
    expansion: &mut [f64; DETERMINANT_TERM_COUNT],
    length: usize,
    term: f64,
) -> usize {
    let mut carry = term;
    let mut output_length = 0;

    // Each read emits at most one residual, so writes never overtake unread
    // components. Six input products need at most six expansion components.
    for read_index in 0..length {
        let component = expansion[read_index];
        let (sum, residual) = general_two_sum(carry, component);

        if residual != 0.0 {
            expansion[output_length] = residual;
            output_length += 1;
        }

        carry = sum;
    }

    if carry != 0.0 {
        expansion[output_length] = carry;
        output_length += 1;
    }

    output_length
}

fn general_two_sum(left: f64, right: f64) -> (f64, f64) {
    let sum = left + right;
    let right_virtual = sum - left;
    let residual = (left - (sum - right_virtual)) + (right - right_virtual);

    (sum, residual)
}
