using System.Reflection;
using OpenUnity.Interop;
using UnityEngine;

namespace OpenUnity.Hosting;

internal sealed class SceneController : IHostedScript, IDisposable
{
    private readonly RuntimeContext context;
    private readonly LifecycleDispatcher dispatcher;
    private readonly ScenePlan plan;
    private readonly List<GameObject> owners = new();
    private readonly List<long> scripts = new();
    private long scene;
    private bool published;
    private (int Source, double Step)? timing;

    internal SceneController(Assembly assembly, byte[] payload)
    {
        RuntimeContext.RequireEngineAccess();
        uint version = OpenUnityNative.ou_scene_abi_version();
        if (version != OpenUnityNative.SceneAbiVersion)
            throw new NotSupportedException($"OpenUnity scene ABI 1.0 is required; received 0x{version:X8}.");
        context = RuntimeContext.Shared;
        context.RequireTransform();
        dispatcher = context.Lifecycle;
        dispatcher.CheckThread();
        plan = ScenePlan.Parse(payload, assembly, dispatcher);
    }

    internal SceneController(Assembly assembly, byte[] payload, int source, double step)
        : this(assembly, payload)
    {
        if (!double.IsFinite(step) ||
            (source == 0 ? step != 0 : source != 1 || step < 0.000001 || step > 60))
            throw new ArgumentException("Invalid timed scene source or step.");
        foreach (ScenePlan.Script script in plan.Scripts)
            dispatcher.PreflightTimedType(script.Type);
        timing = (source, step);
    }

    public long Initialize(long value)
    {
        dispatcher.CheckThread();
        if (timing is { } settings)
            dispatcher.ConfigureTime(settings.Source, settings.Step);
        scene = plan.Stage(context);
        if (scene <= 0)
        {
            long status = scene;
            scene = 0;
            throw RuntimeContext.Failure("scene staging", status);
        }
        try
        {
            Dictionary<long, UnityEngine.Object> objects = BindSceneObjects();
            RestoreScalarFields(objects);
            RestoreReferenceFields(objects);
            RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_scene_publish(runtime, scene)), "scene publication");
            published = true;
        }
        catch (Exception failure)
        {
            ReleaseAfterFailure(failure);
            throw;
        }
        // Callback failures leave the published graph owned for explicit silent cleanup.
        dispatcher.PumpMutations();
        return 0;
    }

    public long Tick(long value) => timing.HasValue ? dispatcher.TickTimed() : dispatcher.Tick();

    public long Shutdown(long value)
    {
        dispatcher.CheckThread();
        try
        {
            RequireSuccess(context.Invoke(runtime => OpenUnityNative.ou_scene_destroy(runtime, scene)),
                "scene destruction request");
            dispatcher.FlushDestroy(timing.HasValue);
        }
        catch (Exception failure)
        {
            ReleaseAfterFailure(failure);
            throw;
        }
        Dispose();
        return 0;
    }

    private void ReleaseAfterFailure(Exception failure)
    {
        try
        {
            Dispose();
        }
        catch (Exception cleanup)
        {
            throw new AggregateException("Scene operation and ownership cleanup failed.", failure, cleanup);
        }
    }

    public void Dispose()
    {
        dispatcher.CheckThread();
        List<Exception> failures = new();
        if (scene != 0)
        {
            try
            {
                RequireSuccess(context.Invoke(runtime => published
                    ? OpenUnityNative.ou_scene_release(runtime, scene)
                    : OpenUnityNative.ou_scene_abort(runtime, scene)), "scene ownership release");
                scene = 0;
            }
            catch (Exception failure)
            {
                failures.Add(failure);
            }
        }
        try
        {
            dispatcher.ReleaseSceneBindings(scripts, owners);
            scripts.Clear();
            owners.Clear();
        }
        catch (Exception failure)
        {
            failures.Add(failure);
        }
        if (failures.Count != 0)
            throw new AggregateException("Scene ownership cleanup failed.", failures);
    }

    private Dictionary<long, UnityEngine.Object> BindSceneObjects()
    {
        Dictionary<long, UnityEngine.Object> objects = new();
        foreach (ScenePlan.Owner item in plan.Owners)
        {
            GameObject owner = GameObject.WrapSceneOwner(context, Resolve(item.Id), Resolve(item.Transform));
            owners.Add(owner);
            objects.Add(item.Id, owner);
            objects.Add(item.Transform, owner.SceneTransform);
        }
        foreach (ScenePlan.Script item in plan.Scripts)
        {
            long identity = Resolve(item.Id);
            scripts.Add(identity);
            objects.Add(item.Id, dispatcher.ConstructSceneScript((GameObject)objects[item.Owner], identity, item.Type));
        }
        return objects;
    }

    private void RestoreScalarFields(Dictionary<long, UnityEngine.Object> objects)
    {
        foreach (ScenePlan.Script item in plan.Scripts)
        {
            foreach (ScenePlan.Field field in item.Fields)
            {
                if (field.Reference is null)
                    field.Metadata.SetValue(objects[item.Id], field.Value);
            }
        }
    }

    private void RestoreReferenceFields(Dictionary<long, UnityEngine.Object> objects)
    {
        foreach (ScenePlan.Script item in plan.Scripts)
        {
            foreach (ScenePlan.Field field in item.Fields)
            {
                if (field.Reference is long reference)
                    field.Metadata.SetValue(objects[item.Id], reference == 0 ? null : objects[reference]);
            }
        }
    }

    private long Resolve(long localId)
    {
        long identity = context.Invoke(runtime => OpenUnityNative.ou_scene_object(runtime, scene, localId));
        if (identity <= 0)
            throw RuntimeContext.Failure("staged scene identity lookup", identity);
        return identity;
    }

    private static void RequireSuccess(int status, string operation)
    {
        if (status != 0)
            throw RuntimeContext.Failure(operation, status);
    }
}
