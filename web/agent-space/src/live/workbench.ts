import type { CloudWorkbench } from '../cloud/workbench';
import type { LiveSpace } from './controller';

/** One App consumes the admitted host adapter; native-only capabilities stay with LiveSpace. */
export type SpaceLive = LiveSpace | CloudWorkbench;
