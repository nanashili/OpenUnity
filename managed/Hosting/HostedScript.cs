namespace OpenUnity.Hosting;

/// <summary>A script contract owned by one hosted session.</summary>
public interface IHostedScript
{
    long Initialize(long value);
    long Tick(long value);
    long Shutdown(long value);
}
