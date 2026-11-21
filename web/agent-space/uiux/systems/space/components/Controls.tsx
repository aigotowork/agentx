/** @aiPurpose Native semantic controls with compact Space roles and consistent focus/disabled behavior. */
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> { variant?: "default" | "primary" | "icon" }
/** @aiPurpose Invoke one deliberate action with native keyboard and disabled semantics.
 * @aiUsage Use primary for submission, icon for named compact actions; native form submission requires type=submit explicitly.
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button({ variant = "default", className = "", type = "button", ...props }, ref) {
  const classes = `space-button ${variant === "icon" ? "icon-button" : ""} ${className}`;
  return <button {...props} ref={ref} type={type} className={classes} data-variant={variant} />;
});
/** @aiPurpose Display an operation or availability result adjacent to its subject.
 * @aiUsage Supply human-readable status; use error for actionable failures and never infer completion from presentation.
 */
export function Feedback({ children, error = false, id }: { children: ReactNode; error?: boolean; id?: string }) {
  return <small id={id} className={error ? "context-error" : "space-feedback"} role={error ? "alert" : "status"}>{children}</small>;
}
