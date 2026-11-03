# BigUint exhaustive search progress

This report tracks a restartable search for integers from 2 through 2^256−1 that are palindromic in binary, decimal, and hexadecimal and are prime. The search enumerates binary/hexadecimal palindrome candidates, then checks decimal palindromicity and primality. Progress is measured against that constrained candidate workload.

- **Status:** running
- **Current bit length:** 140
- **Next candidate position:** 3,200,000,000
- **Candidates processed:** 83,373,509,283 / 49,191,317,547,072,006,819
- **Integers ruled out by binary/hex palindrome constraints through bit length 140:** 1,393,796,574,908,163,946,345,982,392,040,408,060,876,123
- **Overall workload complete:** 0.000000169488%
- **Completed partitions:** 11,749
- **Active batch candidates:** 3,200,000,000–4,800,000,000 (1,600,000,000 scheduled)
- **Active workers:** 16
- **Active batch elapsed:** 0s
- **Triple-palindrome records:** 4
- **Prime classifications:** 3
- **Composite classifications:** 1
- **Unresolved classifications:** 0
- **Active worker time:** 1h 38m 4s
- **Elapsed wall time:** 1h 53m 37s
- **Interrupted batch time observed:** 10m 12s
- **Candidates processed per active search second:** 14,167,877.315
Each candidate position maps to one integer that is palindromic in binary and hexadecimal. Decimal palindromicity and then primality are checked afterward.
- **Started UTC:** 2026-11-03T09:32:15Z
- **Updated UTC:** 2026-11-03T11:25:53Z

Prime classifications in this BigUint run are not, by themselves, rigorous primality proofs. The 256-bit scan is not complete until status is `complete`; unresolved candidates require separate proof or compositeness verification.
