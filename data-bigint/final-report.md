# BigUint exhaustive search progress

This report tracks a restartable search for integers from 2 through 2^256−1 that are palindromic in binary, decimal, and hexadecimal and are prime. The search enumerates binary/hexadecimal palindrome candidates, then checks decimal palindromicity and primality. Progress is measured against that constrained candidate workload.

- **Status:** running
- **Current bit length:** 143
- **Next candidate position:** 0
- **Candidates processed:** 114,533,509,795 / 49,191,317,547,072,006,819
- **Candidates processed per active search second:** 14,053,115.516
- **Integers ruled out by binary/hex palindrome constraints through bit length 143:** 11,150,372,599,265,311,570,767,859,136,324,031,859,742,043
- **Overall workload complete:** 0.000000232833%
- **Completed partitions:** 12,063
- **Active batch candidates:** 0–1,600,000,000 (1,600,000,000 scheduled)
- **Active workers:** 16
- **Active batch elapsed:** 1m
- **Triple-palindrome records:** 4
- **Prime classifications:** 3
- **Composite classifications:** 1
- **Unresolved classifications:** 0
- **Active worker time:** 2h 15m 50s
- **Elapsed wall time:** 2h 36m 51s
- **Estimated remaining time:** 128,176 years, 7 months, 16 days, 9 hours, 55 minutes, 9 seconds
- **Estimated completion date:** 130,118-05-11 22:04:16 UTC
- **Started UTC:** 2026-11-03T09:32:15Z
- **Updated UTC:** 2026-11-03T12:09:07Z

Prime classifications in this BigUint run are not, by themselves, rigorous primality proofs. The 256-bit scan is not complete until status is `complete`; unresolved candidates require separate proof or compositeness verification.
