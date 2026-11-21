# Command surface

`CommandSurface` is the native modal for deliberate user operations. It composes the same `SurfaceHeader` close control as `ObjectWindow`, with independently scrolling fields and persistent Cancel/Submit actions. The caller supplies the exact subject, capability, outcome and native effect; see [the candidate design](candidates.design.md) for styling and verification.

The candidate scene models a destructive confirmation with local feedback. Focus starts on Cancel; Escape and Cancel restore the invoker. Only a distinct Submit can report confirmation.
