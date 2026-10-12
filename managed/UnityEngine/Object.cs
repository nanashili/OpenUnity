using System.Text;
using OpenUnity.Interop;

namespace UnityEngine;

public class Object
{
    private const int MaximumNameBytes = 1_048_576;
    private const int MaximumSizeRetries = 4;
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);
    private readonly RuntimeContext context;
    private readonly long identity;
    private readonly int instanceId;

    // Direct public Object construction is outside the reviewed subset.
    protected Object()
    {
        context = RuntimeContext.Shared;
        identity = context.Invoke(runtime =>
        {
            long created = OpenUnityNative.ou_object_create(runtime);
            if (created <= 0)
                throw RuntimeContext.Failure("object creation", created);
            if (created > int.MaxValue)
            {
                int released = OpenUnityNative.ou_object_destroy(runtime, created);
                if (released != 0)
                    throw RuntimeContext.Failure("out-of-range object cleanup", released);
                throw new OverflowException("OpenUnity object identity exceeds the supported signed 32-bit instance-ID range.");
            }
            return created;
        });
        instanceId = checked((int)identity);
    }

    internal Object(RuntimeContext context, long identity)
    {
        if (identity <= 0)
            throw RuntimeContext.Failure("attached identity", identity);
        this.context = context;
        this.identity = identity;
        instanceId = checked((int)identity);
    }

    internal RuntimeContext Context => context;
    internal long Identity => identity;

    internal static void CheckNative(long status, string operation)
    {
        if (status == -2)
            throw new MissingReferenceException("The OpenUnity object has been destroyed.");
        if (status == -8 || status == -11)
            throw new NotSupportedException($"OpenUnity {operation} is outside the supported Transform subset (native status {status}).");
        if (status < 0)
            throw RuntimeContext.Failure(operation, status);
    }

    public string name
    {
        get => context.Invoke(runtime =>
        {
            for (int sizeChanges = 0; ; sizeChanges++)
            {
                int length = OpenUnityNative.ou_object_name_len(runtime, identity);
                CheckNameRead(length);
                if (length > MaximumNameBytes)
                    throw new InvalidOperationException("OpenUnity returned a name exceeding the 1 MiB byte limit.");
                byte[] buffer = new byte[length];
                int read = OpenUnityNative.ou_object_name_read(runtime, identity, buffer, buffer.Length);
                if (read == -7)
                {
                    if (sizeChanges < MaximumSizeRetries)
                        continue;
                    throw new InvalidOperationException("OpenUnity name changed size more than four times during a read.");
                }
                CheckNameRead(read);
                if (read > buffer.Length)
                    throw new InvalidOperationException("OpenUnity returned an invalid name byte count.");
                try
                {
                    return StrictUtf8.GetString(buffer, 0, read);
                }
                catch (DecoderFallbackException error)
                {
                    throw new InvalidOperationException("OpenUnity returned a name containing invalid UTF-8.", error);
                }
            }
        });
        set
        {
            ArgumentNullException.ThrowIfNull(value);
            byte[] buffer;
            try
            {
                int length = StrictUtf8.GetByteCount(value);
                if (length > MaximumNameBytes)
                    throw new ArgumentException("OpenUnity names cannot exceed 1 MiB of UTF-8 bytes.", nameof(value));
                buffer = StrictUtf8.GetBytes(value);
            }
            catch (EncoderFallbackException error)
            {
                throw new ArgumentException("OpenUnity names must contain valid UTF-16.", nameof(value), error);
            }
            context.Invoke(runtime =>
            {
                int status = OpenUnityNative.ou_object_name_set(runtime, identity, buffer, buffer.Length);
                if (status == -2)
                    throw new MissingReferenceException("The OpenUnity object has been destroyed.");
                if (status != 0)
                    throw RuntimeContext.Failure("name assignment", status);
                return status;
            });
        }
    }

    [Obsolete("Use GetEntityId instead.", true)]
    public int GetInstanceID() => instanceId;

    public override int GetHashCode() => instanceId;

    public override bool Equals(object? other) => other is null
        ? !IsAlive()
        : other is Object obj && this == obj;

    public static bool operator ==(Object? left, Object? right)
    {
        if (ReferenceEquals(left, null))
            return ReferenceEquals(right, null) || !right.IsAlive();
        if (ReferenceEquals(right, null))
            return !left.IsAlive();
        return left.identity == right.identity;
    }

    public static bool operator !=(Object? left, Object? right) => !(left == right);

    public static implicit operator bool(Object? value) =>
        !ReferenceEquals(value, null) && value.IsAlive();

    public static void DestroyImmediate(Object? obj)
    {
        if (ReferenceEquals(obj, null))
            return;
        obj.context.Invoke(runtime =>
        {
            int status = OpenUnityNative.ou_object_destroy(runtime, obj.identity);
            if (status == -8)
                throw new NotSupportedException("OpenUnity does not support destroying a mandatory Transform directly.");
            // Only identities created and retained by this wrapper reach here.
            if (status != 0 && status != -2)
                throw RuntimeContext.Failure("immediate destruction", status);
            return status;
        });
    }

    public static void Destroy(Object? obj)
    {
        if (ReferenceEquals(obj, null))
            return;
        obj.context.Lifecycle.Mutate(runtime =>
        {
            int status = OpenUnityNative.ou_object_destroy_deferred(runtime, obj.identity);
            return status == -2 ? 0 : status;
        }, "deferred destruction");
    }

    private bool IsAlive() => context.Invoke(runtime =>
    {
        int status = OpenUnityNative.ou_object_is_alive(runtime, identity);
        return status switch
        {
            1 => true,
            0 => false,
            _ => throw RuntimeContext.Failure("liveness query", status)
        };
    });

    private static void CheckNameRead(int status)
    {
        if (status == -2)
            throw new MissingReferenceException("The OpenUnity object has been destroyed.");
        if (status < 0)
            throw RuntimeContext.Failure("name read", status);
    }
}
