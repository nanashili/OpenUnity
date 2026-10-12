using System.Reflection;
using System.Runtime.Loader;
using System.Text;

namespace OpenUnity.Hosting;

public static unsafe partial class EntryPoints
{
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    private static Assembly LoadUserAssembly(string path)
    {
        string fullPath = Path.GetFullPath(path);
        AssemblyLoadContext context = AssemblyLoadContext.GetLoadContext(typeof(EntryPoints).Assembly)
            ?? AssemblyLoadContext.Default;
        return context.Assemblies.FirstOrDefault(candidate =>
            !string.IsNullOrEmpty(candidate.Location) &&
            string.Equals(Path.GetFullPath(candidate.Location), fullPath, StringComparison.Ordinal))
            ?? context.LoadFromAssemblyPath(fullPath);
    }

    private static long CreateSession(byte* assemblyPathUtf8, byte* typeNameUtf8, bool gameplay)
    {
        try
        {
            if (assemblyPathUtf8 is null || typeNameUtf8 is null)
                return InvalidInput;

            string? path = ReadUtf8(assemblyPathUtf8);
            string? typeName = ReadUtf8(typeNameUtf8);
            if (string.IsNullOrWhiteSpace(path) || string.IsNullOrWhiteSpace(typeName) ||
                !Path.IsPathFullyQualified(path) || !File.Exists(path))
                return InvalidInput;

            Assembly assembly = LoadUserAssembly(path);
            Type? scriptType = assembly.GetType(typeName, throwOnError: false, ignoreCase: false);
            if (scriptType is null || !scriptType.IsClass || scriptType.IsAbstract ||
                scriptType.ContainsGenericParameters || scriptType.GetConstructor(Type.EmptyTypes) is null ||
                (gameplay ? scriptType == typeof(UnityEngine.MonoBehaviour) ||
                    !typeof(UnityEngine.MonoBehaviour).IsAssignableFrom(scriptType) :
                    !typeof(IHostedScript).IsAssignableFrom(scriptType)))
                return InvalidInput;

            IHostedScript? script = gameplay ? new GameplayController(scriptType) :
                Activator.CreateInstance(scriptType) as IHostedScript;
            if (script is null)
                return InvalidInput;

            lock (Gate)
            {
                if (_lastHandle == long.MaxValue)
                    return Exhausted;
                long handle = ++_lastHandle;
                Sessions.Add(handle, new Session(script));
                return handle;
            }
        }
        catch (ArgumentException)
        {
            return InvalidInput;
        }
        catch (FileNotFoundException)
        {
            return InvalidInput;
        }
        catch (BadImageFormatException)
        {
            return InvalidInput;
        }
        catch (TypeLoadException)
        {
            return InvalidInput;
        }
        catch
        {
            return CallbackFailed;
        }
    }

    private static string? ReadUtf8(byte* pointer)
    {
        int length = 0;
        while (pointer[length] != 0)
            length++;
        return StrictUtf8.GetString(pointer, length);
    }
}
