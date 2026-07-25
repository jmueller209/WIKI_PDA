```bash
================ DETAILED PARSING METRICS ================
Total Lines Read:                 1000000
Total Lines Skipped:              518072   # Dropped: failed wiki, concept, & special index filters.
Unique QIDs Found:                999971
Unique QIDs Added (Omni Search):  481927   # Written to the primary text search index.
P-IDs Found:                      29
P-IDs Added (Psearch):            1
Q-IDs Added without Wiki Entry:   247175   # Rescued: Kept via P31 concepts or special indices, lacking wiki articles.
--------------------------------------------------------
SPECIAL INDEX: GLOBE COORDINATES
  Total Count:                    198919   # Total items successfully added to this index.
  Saved by Index (Independent):   186694   # Rescued exclusively because 'include_all_matches_in_globe' is true.
  Missing Omni-Search Terms:      184684   # Has coordinates, but ZERO labels/aliases in target languages.
SPECIAL INDEX: TEMPORAL
  Total Count:                    449921
  Saved by Index (Independent):   109672
  Missing Omni-Search Terms:      303105
SPECIAL INDEX: ASTRONOMICAL
  Total Count:                    1567
  Saved by Index (Independent):   1567
  Missing Omni-Search Terms:      0        # 0 expected: bots auto-fill scientific catalog names across languages.
--------------------------------------------------------
NUMBER OF QIDS THAT HAVE AT LEAST ONE ENTRY IN GIVEN WIKI IN YOUR CONFIGURED LANGUAGES:
   - wiki: 943 items                      # Mapped to Wikimedia sites in target languages (e.g., 'ang').
   - wikiquote: 6 items
   - wiktionary: 6 items
   - wikibooks: 4 items
   - wikisource: 1 items
--------------------------------------------------------
TOP INCLUDED CONCEPTS (P31 / Filter):
   - Q5: 214134 items                     # Frequent P31 ontological classes (e.g., Q5 = human) triggering inclusion.
   - Q11424: 12917 items
   - Q486972: 10917 items
   - Q7725634: 7689 items
   - Q515: 859 items
   - Q3305213: 847 items
--------------------------------------------------------
TOP 25 MOST USED PROPERTIES IN METADATA:
 1. P31: 1032141 times                    # Raw frequency of properties attached to the final dataset.
 2. P646: 450888 times                    # Reveals data shape (e.g., high P625 indicates spatial density, P569 temporal).
 3. P106: 409503 times
 4. P1082: 353228 times
 5. P17: 313537 times
 6. P373: 297082 times
 7. P2671: 285709 times
 8. P18: 259444 times
 9. P131: 235630 times
10. P27: 219859 times
11. P21: 217771 times
12. P569: 216722 times
13. P735: 214219 times
14. P625: 198919 times
15. P19: 169018 times
16. P734: 160657 times
17. P214: 150867 times
18. P1412: 144050 times
19. P54: 137028 times
20. P971: 126330 times
21. P570: 122189 times
22. P166: 113564 times
23. P161: 111192 times
24. P244: 100798 times
25. P69: 100714 times
========================================================
