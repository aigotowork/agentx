/** @aiPurpose Render actual owned resources for inspection without product effects. */
import {FeedbackControls} from './feedback.web';
import {useDesignSystem} from '../platforms/web/studio.context';
/** @aiPurpose Provide scenes presentation behavior.
 * @aiUsage Use within the Inspector host and its themed resource boundary. */
export const scenes={default:()=>{const {theme,feedback}=useDesignSystem();return <FeedbackControls theme={theme} feedback={feedback}/>;}};
