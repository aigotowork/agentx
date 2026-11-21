/** @aiPurpose Expose maintained Space presentation resources; never import fixtures or native effect adapters here. */
export { OfficeLayout } from "./layouts/OfficeLayout";
export { SpaceLayout } from "./layouts/SpaceLayout";
export { ChatLayout } from "./layouts/ChatLayout";
export { WorkspaceLayout, ConversationLayout } from "./layouts/WorkspaceLayout";
export { ConversationComposer, type ConversationComposerProps } from "./components/ConversationComposer";
export { Attachments } from "./components/Attachments";
export { FactButton } from "./components/FactButton";
export { SpaceInteractions, useOverlay, useSpaceShortcuts } from "./components/SpaceInteractions";
export { Button, Feedback } from "./components/Controls";
export { ConversationBlock } from "./blocks/ConversationBlock";
export { SceneControls } from "./blocks/SceneControls";
export { SceneObjectLayer, ActorAnchor } from "./blocks/SceneObjectLayer";
export { ObjectWindow } from "./components/ObjectWindow";
export { SpaceTheme } from "./components/SpaceTheme";
export { SpaceStyleSwitch } from "./components/SpaceStyleSwitch";
export { SPACE_STYLES, parseSpaceStyle, type SpaceStyle } from "./components/presentation";
export { CommandSurface } from "./components/CommandSurface";
export { HumanMailboxBlock, type MailboxFeed } from "./blocks/HumanMailboxBlock";
