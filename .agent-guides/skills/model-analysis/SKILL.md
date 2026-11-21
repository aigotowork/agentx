---
name: model-analysis
description: Derive or inspect a behavioral model for a nontrivial feature, protocol or module using scenarios, data, state, authority and counterexamples. Use when relationships or lifecycle behavior are uncertain.
---

# Analyze a model

Start with the actual question and one scenario that can answer it. Reuse the existing authoritative model and the consumer's working record. A local bug does not require modeling an entire product.

1. Identify the actor's intended result, available inputs, resource identity, relevant authority and observable completion. Separate known source facts, assumptions and unanswered questions.
2. Follow one concrete path through the current system. Name who owns each concept, operation, data source and effect. Keep reading current declarations and callers where a relationship is uncertain.
3. Choose the smallest useful view: entity/relationship constraints for data; `state + event + guard -> next state + effects` for lifecycle; operation/input/result contracts for boundaries; a dependency diagram for composition. Keep views consistent with the same scenario.
4. Try the case that threatens the conclusion: missing data, stale identity, cancellation, failure after an effect, duplicate input or changed version. Record what the model predicts and what evidence would distinguish alternatives. Do not interpret a lost reply as permission to repeat an effect.
5. Resolve the current decision in the owning model or contract, preserving original evidence and explicit uncertainties. If implementation is in scope, change its responsible owner and verify the meaningful outcome.

Models explain behavior; they do not prove it happened. Select a bounded real check or an independent oracle when the conclusion depends on execution. Stop when the requested decision is supported and remaining limits are explicit. No dedicated modeling CLI is required; use the consumer's existing modeling tool only when its public contract fits.
