using System.Reflection;
using System.Text;
using System.Text.Json;
using UnityEngine;

namespace OpenUnity.Hosting;

// This is the independently specified transfer format, not a Unity scene parser.
internal sealed partial class ScenePlan
{
    internal const int MaximumBytes = 16 * 1024 * 1024;
    private const int MaximumObjects = 10_000;
    private static readonly UTF8Encoding Utf8 = new(false, true);
    internal sealed record Owner(long Id, long Transform);
    internal sealed record Field(FieldInfo Metadata, object? Value, long? Reference);
    internal sealed record Script(long Id, long Owner, Type Type, Field[] Fields);
    internal Owner[] Owners { get; }
    internal Script[] Scripts { get; }
    private readonly byte[] payload;

    private ScenePlan(byte[] payload, Owner[] owners, Script[] scripts)
    {
        this.payload = payload;
        Owners = owners;
        Scripts = scripts;
    }

    internal long Stage(RuntimeContext context) => context.Invoke(runtime =>
        OpenUnity.Interop.OpenUnityNative.ou_scene_stage(runtime, payload, payload.Length));

    internal static ScenePlan Parse(byte[] payload, Assembly assembly, LifecycleDispatcher dispatcher)
    {
        ValidatePayload(payload, assembly);
        using JsonDocument document = JsonDocument.Parse(payload, new JsonDocumentOptions { MaxDepth = 64 });
        PlanArrays arrays = ReadEnvelope(document.RootElement);
        Dictionary<long, Type> identities = new();
        Dictionary<long, long> transformOwners = ReadTransforms(arrays.Transforms, identities);
        List<Owner> owners = ReadOwners(arrays.GameObjects, identities, transformOwners);
        ReadIds(arrays.Root.GetProperty("roots"));
        List<(JsonElement Data, Type Type)> scriptTypes =
            ReadScriptBindings(arrays.Scripts, assembly, dispatcher, identities);
        List<Script> prepared = ReadScripts(scriptTypes, identities);
        return new ScenePlan(payload, owners.ToArray(), prepared.ToArray());
    }

    private static void ValidatePayload(byte[] payload, Assembly assembly)
    {
        if (payload.Length is <= 0 or > MaximumBytes)
            throw Invalid("Scene plan byte limit exceeded.");
        if (assembly.GetName().Name != "Assembly-CSharp")
            throw Invalid("Scene scripts require the supplied Assembly-CSharp assembly identity.");
        _ = Utf8.GetString(payload);
    }

    private static PlanArrays ReadEnvelope(JsonElement element)
    {
        ValidateStringsAndDuplicates(element);
        JsonElement root = Shape(element, "schema_version", "scene_name", "roots", "game_objects", "transforms", "scripts");
        if (Integer(root.GetProperty("schema_version")) != 1)
            throw Invalid("Unsupported scene plan version.");
        _ = Text(root.GetProperty("scene_name"));
        JsonElement gameObjects = Array(root.GetProperty("game_objects"));
        JsonElement transforms = Array(root.GetProperty("transforms"));
        JsonElement scripts = Array(root.GetProperty("scripts"));
        if ((long)gameObjects.GetArrayLength() + transforms.GetArrayLength() + scripts.GetArrayLength() > MaximumObjects)
            throw Invalid("Scene object limit exceeded.");
        return new PlanArrays(root, gameObjects, transforms, scripts);
    }

    private readonly record struct PlanArrays(
        JsonElement Root, JsonElement GameObjects, JsonElement Transforms, JsonElement Scripts);
}
