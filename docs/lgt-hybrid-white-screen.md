# Hybrid white screen investigation

On the Mac AVD the LGT archive e68b1c8aef85c584dc6e5e225f0c226641b23f6c523bf0f59eb662151f9cf954 shows the usage notice, then becomes white after OK. Isolated fresh-start diagnostics reproduce this with copies of the current saves. No uncaught Java exception or host crash appears. This is not the Battle Monster StringBuffer failure.

The guest loads certi.pzx, frees its resources, closes invalid network sockets, cancels timer 0x1504c00, calls service 0x6a with (0x5001,2,0), then service Unk11 with (0,0x5001,2,0), flushes a white framebuffer and produces no more frames. These service handlers are stubs. No repeated connection loop is present in the observed trace.

At 0x783c8 a helper at 0x8a279 is called on the object at this+0x6c. Return 2 selects object destruction and a call to 0x3a29d with argument 2. That function invokes cleanup at 0x39fe0, then sends (0x5001,2,0) via wrapper 0x2f685; LR at the service is 0x2f693. Return 0 follows another path. The exact meaning of status 2 and the platform service remains unconfirmed. certi resource naming alone does not prove authentication failure.

No production fix applied: do not force this status to zero, restart the canceled timer, or invent a successful platform response. Next investigation should identify why the object reports 2 and establish the service contract. Diagnostics were removed and the regular APK restored. User saves were not reset.

## Activation barrier confirmed

An isolated empty-save run reaches a green prompt: game execution requires authentication, with Yes/No choices. After waiting for the prompt and choosing Yes, the regular native app with the current offline network backend displays authentication failure and a retry-later message. This is not an observed successful network exchange: net::connect deliberately completes with offline error. No claim is made about the availability of the original server.

The existing save contains hbt.dat, hbtopt.dat and audio.adt records. audio.adt is a 60-byte activation record despite its filename: native routine 0x894f0 reads it, transforms its bytes and tests identity/date/counter fields in 0x89574. The user record contains empty identity bytes and nonzero attempt/date fields. Thus deleting all game progress would be disproportionate and would not solve the network barrier: an empty record merely restores the authentication prompt.

No activation-success response or forced state transition was added. Continuing beyond mandatory activation requires either valid user-supplied activation data or a separately agreed per-game compatibility change; it cannot be inferred as a shared API fix. All experiments used isolated save copies. Temporary changes to the instrumentation key spacing were restored.
