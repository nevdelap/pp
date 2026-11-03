# Mathematical derivation and exhaustiveness proof

Use bit positions `b_i` from least significant (`i = 0`) to most significant (`i = L-1`). A number has bit length `L` exactly when `b_(L-1)=1`.

## Exact simultaneous base-2/base-16 constraints

Let `H = ceil(L/4)` be the number of hexadecimal digits and let `r = L - 4(H-1)`, so `1 <= r <= 4` is the width of the leading hexadecimal digit. Hex digit `d_j`, counted from the least significant end, is the integer represented by bits `b_(4j)..b_(4j+3)` where those positions exist.

Binary palindromicity is exactly the set of equalities

`b_i = b_(L-1-i)` for every `0 <= i < L`.

Hexadecimal palindromicity is exactly the following additional constraints:

* For each paired pair of full digits `1 <= j < H/2` and each `0 <= k < 4`, `b_(4j+k) = b_(4(H-1-j)+k)`.
* For the outer pair, when `H >= 2`, the leading digit has only `r` meaningful bits. Equality of the leading and trailing digit values requires `b_k = b_(4(H-1)+k)` for `0 <= k < r`, and `b_k=0` for `r <= k < 4`.
* A middle hexadecimal digit is unconstrained by digit reversal itself; its binary-bit equalities still apply.

These are all equalities between bit variables and fixed-zero conditions. The implementation builds their equivalence classes with a disjoint-set structure, fixes the top-bit class to one and the forced-zero class to zero, and enumerates every assignment to the remaining classes. Each assignment is unique because every remaining class contains at least one bit, and different assignments change at least one output bit. Conversely, any integer satisfying both palindrome definitions obeys every listed constraint and therefore appears in exactly one assignment. This covers all bit lengths `2..=128`; leading partial hexadecimal digits are explicitly handled.

## Proven exclusions used for prime classification

For an even binary length, paired binary digits occur at opposite parity positions. Since `2 == -1 (mod 3)`, every even-length binary palindrome is divisible by 3. The only prime exception is the number 3 itself.

For an even hexadecimal digit count, paired hexadecimal digits occur at opposite parity positions. Since `16 == -1 (mod 17)`, every such hexadecimal palindrome is divisible by 17. The only prime exception is 17; it is not decimal-palindromic.

An even-length decimal palindrome is divisible by 11 by the alternating-sum test; the only prime exception is 11, which is not binary-palindromic. These rules are used only after a candidate has passed all three palindrome checks, so the stored audit still includes every simultaneous palindrome.

The full work space is the union of the disjoint assignment spaces for every `L=2..128`. Length one contributes no input integer >=2 with a binary-palindromic canonical representation (2 is `10`, not a palindrome); 3 is included at length two. Values above `2^128-1` cannot be generated because the highest bit is at most 127.

## Independent checks

`reference_pal` converts to a vector of base digits and compares that vector with its reverse. Tests compare it against all three optimized checks and compare generated spaces against direct integer enumeration for small bit lengths. The coverage verifier checks every expected partition index and its declared half-open assignment interval.
