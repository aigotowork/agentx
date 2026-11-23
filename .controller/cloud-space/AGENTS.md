# Cloud Space

Read the [wire contract](wire.md) before changing a Cloud Space resource or action. Cloud authorization, operation receipts and exact host identities belong to the cloud APIs. The public HTTP SDK owns browser transport. The existing Space app owns roster, selection and interaction. Deployment operators also use [the container guide](../../deploy/README.md).

The [verified cloud image build](https://github.com/aigotowork/agentx/actions/runs/44971452694) includes SDK, browser and container smoke checks. The deployed image from commit `e0fc703` was also checked in a public HTTPS browser with both access modes after rollout. A live launch on the remote host was followed by an exact stop with a successful receipt. The roster is a snapshot. After a queued launch, check the receipt and refresh to see registration.

For local/cloud workbench convergence, see the [native conversation contract](shared/native.task.md), [shared workbench task](shared/web.task.md) and [host answer forwarding](shared/provider.task.md). The operator workflow runs through one App and one conversation/launch UI. Transport selection owns cloud authentication, host routing and exact agent addressing.

For event-driven reads, active transcript, launch facts and target work directories see [refresh design](shared/refresh.task.md), [transcript delivery](shared/transcript.task.md) and [directory launch](shared/directory.task.md).
