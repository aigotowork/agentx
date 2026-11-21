# Space Layout contract

`SpaceLayout` owns the navigation and working content boundary. Its parent retains selection, exact identity, camera, and drafts. Wide screens place navigation beside the content; narrow screens use the existing local disclosure and return behavior.

ConversationLayout owns a four-row grid: intrinsic identity, bounded feedback, one `minmax(0,1fr)` scrolling record/context body, and an intrinsic primary composer. Secondary launch context belongs inside body scrolling; it cannot reserve space at the expense of input/Send. `styles/conversation-layout.css` is the sole active owner of those row allocations. Historical footer shrink/percentage limits and the history floor are removed, rather than countered by viewport-specific minimums. The layout is an explicitly sized `space-conversation` container, so local disclosure limits follow the actual reader height.
