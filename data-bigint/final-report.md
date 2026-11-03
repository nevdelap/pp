# BigUint exhaustive search progress

This report tracks a restartable search for integers from 2 through 2^256−1 that are palindromic in binary, decimal, and hexadecimal and are prime. The search enumerates binary/hexadecimal palindrome candidates, then checks decimal palindromicity and primality. Progress is measured against that constrained candidate workload.

- **Status:** running
- **Current bit length:** 140
- **Next candidate position:** 20,800,000,000
- **Candidates processed:** 100,973,509,283 / 49,191,317,547,072,006,819
- **Candidates processed per active search second:** 14,184,615.163
- **Integers ruled out by binary/hex palindrome constraints through bit length 140:** 1,393,796,574,908,163,946,345,982,392,040,408,060,876,123
- **Overall workload complete:** 0.000000205267%
- **Completed partitions:** 11,925
- **Active batch candidates:** 20,800,000,000–22,400,000,000 (1,600,000,000 scheduled)
- **Active workers:** 16
- **Active batch elapsed:** 1m 30s
- **Triple-palindrome records:** 4
- **Prime classifications:** 3
- **Composite classifications:** 1
- **Unresolved classifications:** 0
- **Active worker time:** 1h 58m 38s
- **Elapsed wall time:** 2h 17m 33s
- **Estimated remaining time:** 127,499 years, 11 months, 29 days, 8 hours, 18 minutes, 29 seconds
- **Estimated completion date:** 129,442-03-04 20:08:18 UTC
Each candidate position maps to one integer that is palindromic in binary and hexadecimal. Decimal palindromicity and then primality are checked afterward.
- **Started UTC:** 2026-11-03T09:32:15Z
- **Updated UTC:** 2026-11-03T11:49:49Z

Prime classifications in this BigUint run are not, by themselves, rigorous primality proofs. The 256-bit scan is not complete until status is `complete`; unresolved candidates require separate proof or compositeness verification.
