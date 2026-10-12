using OpenUnity.Interop;

namespace UnityEngine;

internal sealed class RigidbodyBinding : INativeComponentFactoryBinding
{
    internal RigidbodyBinding()
    {
    }

    public Type DeclaredType => typeof(Rigidbody);
    public NativeComponentBindingCategory Category => NativeComponentBindingCategory.CreateAndQuery;

    public Component CreateAttached(NativeComponentIdentity identity) =>
        new Rigidbody(identity.Context, identity.Identity, identity.Owner);

    public long Reserve(RuntimeContext context, GameObject owner)
    {
        bool reserveCallStarted = false;
        try
        {
            return context.Invoke(runtime => ReserveWithinInvocation(
                runtime,
                owner.Identity,
                () => reserveCallStarted = true));
        }
        catch (NativeComponentKnownRejectionException)
        {
            throw;
        }
        catch (NativeComponentReservationNotStartedException)
        {
            throw;
        }
        catch (ReserveCallFailureException)
        {
            throw;
        }
        catch (Exception failure) when (reserveCallStarted)
        {
            throw new ReserveCallFailureException(failure);
        }
        catch (Exception failure)
        {
            throw new NativeComponentReservationNotStartedException(failure);
        }
    }

    private static long ReserveWithinInvocation(
        long runtime,
        long owner,
        Action markReserveCallStarted)
    {
        uint abi;
        try
        {
            abi = OpenUnityNative.ou_physics_motion_abi_version();
        }
        catch (Exception failure)
        {
            throw new NativeComponentReservationNotStartedException(failure);
        }

        if (abi != OpenUnityNative.PhysicsMotionAbiVersion)
            throw new NativeComponentReservationNotStartedException(
                new NotSupportedException(
                    $"OpenUnity Physics motion ABI 1.0 is required; received 0x{abi:X8}."));

        long identity;
        markReserveCallStarted();
        try
        {
            identity = OpenUnityNative.ou_rigidbody_reserve(runtime, owner);
        }
        catch (Exception failure)
        {
            throw new ReserveCallFailureException(failure);
        }

        if (identity < 0)
            throw new NativeComponentKnownRejectionException(
                StatusFailure("Rigidbody reservation", identity));
        return identity;
    }

    public void Commit(RuntimeContext context, long identity) =>
        RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_rigidbody_commit(runtime, identity)),
            "Rigidbody commit");

    public void Abort(RuntimeContext context, long identity) =>
        RequireSuccess(context.InvokeCleanup(runtime => OpenUnityNative.ou_rigidbody_abort(runtime, identity)),
            "Rigidbody reservation cleanup");

    private static void RequireSuccess(int status, string operation)
    {
        if (status == 0)
            return;
        throw StatusFailure(operation, status);
    }

    private static Exception StatusFailure(string operation, long status)
    {
        if (status == -2)
            return new MissingReferenceException($"OpenUnity {operation} failed because the Rigidbody was destroyed (native status -2).");
        if (status is -8 or -18)
            return new NotSupportedException($"OpenUnity {operation} is outside the supported Rigidbody subset (native status {status}).");
        return RuntimeContext.Failure(operation, status);
    }

    private sealed class ReserveCallFailureException(Exception failure)
        : Exception("The Rigidbody reserve call failed without returning a native status.", failure);
}
