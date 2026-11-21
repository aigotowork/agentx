/** @aiPurpose Define presentation intents and draft ownership without copying native DTOs. */
import type { ApplyCall, BriefInput, Candidate, Change, Descriptor, Failure, OperatorCommand, Role, ThreadMessage, WorkDetail, WorkRef } from "./api/native";

export type ResourceView = "work" | "agents" | "results" | "checks";
export type DetailTab = "brief" | "results" | "checks" | "messages" | "edit";
export type WorkFilter = "all" | "mine" | "checking" | "active";
export interface Selection { workId: string | null; tab: DetailTab; resultId: string | null; questionId: string | null; participant: string | null }
export interface BriefDraft { input: BriefInput; base: BriefInput | null; revision: bigint | null; implementer: string; checker: string; touched: boolean; dirty: boolean }
export type CommandState = { status: "idle" } | { status: "pending" | "unknown"; call: ApplyCall; label: string } | { status: "restored"; commandId: string; generation: string; scope: string; project: string } | { status: "committed"; change: Change; label: string } | { status: "rejected"; failure: Failure | string; workId: string | null; label: string };
export interface ResourceData { descriptor: Descriptor | null; details: WorkDetail[]; candidates: Candidate[]; loading: boolean; error: string | null; observedAt: string | null; next: string | null; candidateNext: string | null }
export type CommandIntent = (command: OperatorCommand, work: WorkDetail | null, label: string, expectedRevision?: bigint) => Promise<boolean>;
export interface CollectionIntent { select: (workId: string, tab?: DetailTab, resourceId?: string) => void; inspectParticipant: (key: string) => void; create: () => void; refresh: () => void; more: () => void }
export interface EditorContract { draft: BriefDraft; work: WorkDetail | null; candidates: Candidate[]; descriptor: Descriptor; blocked: string | null; onDraft: (draft: BriefDraft) => void; onCommand: CommandIntent; onDiscard: () => void; onRefreshCandidates: () => void }
export interface ConversationDraft { answer: string; amendment: boolean; amendmentBrief: BriefInput | null; note: string; roles: Role[]; recipients: string; acknowledged: boolean }
export interface ConversationContract { candidates: Candidate[]; detail: WorkDetail; draft: ConversationDraft; questionId: string | null; blocked: string | null; limits: Descriptor["limits"]; onDraft: (draft: ConversationDraft) => void; onQuestion: (id: string) => void; onCommand: CommandIntent; onEditRoles: () => void }
export interface EvidenceContract { detail: WorkDetail; resultId: string | null; blocked: string | null; textLimit: bigint; onResult: (id: string) => void; onCommand: CommandIntent }
export interface PendingReference { commandId: string; generation: string; scope: string; project: string; work: WorkRef | null }
export interface ThreadState { workId: string | null; generation: string | null; items: ThreadMessage[]; next: bigint | null; loading: boolean; error: string | null }
