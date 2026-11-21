import type {StatsDirectoryAgent} from './stats-types';
import type {DirectoryHealth} from '../shared/domain/source-health';
/** Query-side directory projection. Acquisition/identity admission belongs to the future native adapter. */
export interface StatsDirectory {agents:StatsDirectoryAgent[];loaded:boolean;truncated:boolean;health:DirectoryHealth}
