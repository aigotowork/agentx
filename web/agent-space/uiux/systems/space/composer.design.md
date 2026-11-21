# Conversation Composer contract

The Composer is controlled. It owns input mechanics, IME-safe keyboard submission, attachment gestures, and accessible status. The caller owns draft and attachment state and clears only after a successful operation receipt that still corresponds to the submitted value.

The primary input and toolbar remain intrinsic footer content. Message options occupy a compact accessible disclosure in the toolbar; attachment count has an explicit local disclosure when attachments exist. Their preview/fields are bounded overlays above the toolbar, with local overflow and the shared Escape/return owner. Supplied error text and long toolbar status have bounded local reading regions; they cannot turn the entire primary composer into a scroll container. File registration/removal, controlled option values and native send semantics stay in the product adapter.
