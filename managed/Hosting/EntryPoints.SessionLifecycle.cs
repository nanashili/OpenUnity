using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace OpenUnity.Hosting;

public static unsafe partial class EntryPoints
{
    private const int InitializeOpcode = 1;
    private const int TickOpcode = 2;
    private const int ShutdownOpcode = 3;
    private static readonly object Gate = new();
    private static readonly Dictionary<long, Session> Sessions = new();
    private static long _lastHandle;
    private static int activeSessionOperationDepth;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int Invoke(long handle, int opcode, long value, long* output)
    {
        try
        {
            if (output is null)
                return InvalidInput;

            lock (Gate)
            {
                if (!Sessions.TryGetValue(handle, out Session? session))
                    return InvalidHandle;
                if (opcode is not (InitializeOpcode or TickOpcode or ShutdownOpcode))
                    return InvalidInput;

                activeSessionOperationDepth++;
                try
                {
                    return InvokeSession(session, opcode, value, output);
                }
                finally
                {
                    activeSessionOperationDepth--;
                }
            }
        }
        catch
        {
            return CallbackFailed;
        }
    }

    // Registry ownership and phase transitions remain protected by Gate,
    // including recursive calls made by the current managed callback.
    private static int InvokeSession(Session session, int opcode, long value, long* output)
    {
        if (!TryAdmitInvocation(session, opcode))
            return InvalidLifecycle;

        try
        {
            long result = opcode switch
            {
                InitializeOpcode => session.Script.Initialize(value),
                TickOpcode => session.Script.Tick(value),
                ShutdownOpcode => session.Script.Shutdown(value),
                _ => throw new InvalidOperationException("An unsupported callback was admitted."),
            };

            CompleteInvocation(session, opcode);
            *output = result;
            return Success;
        }
        catch
        {
            if (session.Phase != Phase.Destroyed)
                session.Phase = Phase.Faulted;
            return CallbackFailed;
        }
    }

    private static bool TryAdmitInvocation(Session session, int opcode)
    {
        switch (opcode)
        {
            case InitializeOpcode when session.Phase == Phase.Created:
                session.Phase = Phase.Initializing;
                return true;
            case TickOpcode or ShutdownOpcode when session.Phase == Phase.Initialized:
                return true;
            default:
                return false;
        }
    }

    private static void CompleteInvocation(Session session, int opcode)
    {
        // A callback may have successfully removed its own session. Returning
        // from that callback must preserve its terminal ownership state.
        if (session.Phase == Phase.Destroyed)
            return;

        if (opcode == InitializeOpcode)
            session.Phase = Phase.Initialized;
        else if (opcode == ShutdownOpcode)
            session.Phase = Phase.Shutdown;
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int Destroy(long handle)
    {
        try
        {
            lock (Gate)
            {
                if (!Sessions.TryGetValue(handle, out Session? session))
                    return InvalidHandle;
                if (session.Script is SceneController controller)
                    controller.Dispose();
                Sessions.Remove(handle);
                session.Phase = Phase.Destroyed;
                return Success;
            }
        }
        catch
        {
            return CallbackFailed;
        }
    }

    private enum Phase
    {
        Created,
        Initializing,
        Initialized,
        Shutdown,
        Faulted,
        Destroyed,
    }

    private sealed class Session(IHostedScript script)
    {
        public IHostedScript Script { get; } = script;
        public Phase Phase { get; set; } = Phase.Created;
    }
}
