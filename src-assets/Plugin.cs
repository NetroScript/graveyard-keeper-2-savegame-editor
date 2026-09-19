using System;
using System.Collections;
using System.IO;
using System.Linq;
using BepInEx;
using BepInEx.Configuration;
using UnityEngine;

namespace Gk2.AssetExporter
{
    [BepInPlugin("gk2.saveeditor.assetexporter", "GK2 Asset Exporter", "0.1.0")]
    public sealed class Plugin : BaseUnityPlugin
    {
        private ConfigEntry<KeyboardShortcut> shortcut;
        private ConfigEntry<string> output;
        private bool exporting;

        private void Awake()
        {
            shortcut = Config.Bind("Export", "Shortcut", new KeyboardShortcut(KeyCode.F8), "Export after reaching the main menu or loading a save.");
            output = Config.Bind("Export", "OutputDirectory", "asset-exports", "Absolute path or path relative to BepInEx. Each run creates a new directory.");
            Logger.LogInfo("Asset exporter ready. Press F8 after the game has loaded.");
        }

        private void Update()
        {
            if (!exporting && shortcut.Value.IsDown()) StartCoroutine(Export());
        }

        private IEnumerator Export()
        {
            exporting = true;
            ExportContext context = null;
            // Unity resources and GPU readback must remain on the game's main thread.
            try
            {
                var root = Path.IsPathRooted(output.Value) ? output.Value : Path.Combine(Paths.BepInExRootPath, output.Value);
                var directory = Path.Combine(root, DateTime.UtcNow.ToString("yyyyMMdd-HHmmss") + "-" + Guid.NewGuid().ToString("N").Substring(0, 8));
                try { context = new ExportContext(directory); }
                catch (Exception error)
                {
                    Logger.LogError("Could not initialize export: " + error);
                    yield break;
                }
                Logger.LogInfo("Exporting to " + directory);
                // New modules are discovered automatically; Order declares dependencies.
                var modules = typeof(Plugin).Assembly.GetTypes()
                    .Where(t => !t.IsAbstract && typeof(IExportModule).IsAssignableFrom(t))
                    .Select(t => (IExportModule)Activator.CreateInstance(t))
                    .OrderBy(m => m.Order).ThenBy(m => m.Id, StringComparer.Ordinal).ToArray();
                foreach (var module in modules)
                {
                    Logger.LogInfo("Export module: " + module.Id);
                    var work = module.Export(context);
                    try
                    {
                        while (true)
                        {
                            bool next;
                            try { next = work.MoveNext(); }
                            catch (Exception error) { context.Warn(module.Id, error.ToString()); break; }
                            if (!next) { context.CompletedModules.Add(module.Id); break; }
                            yield return work.Current;
                        }
                    }
                    finally { (work as IDisposable)?.Dispose(); }
                }
                context.Finish();
                Logger.LogInfo($"Export finished: {directory}. Warnings: {context.Warnings.Count}. Check manifest.json.");
            }
            finally
            {
                context?.Dispose();
                exporting = false;
            }
        }
    }

    public interface IExportModule
    {
        string Id { get; }
        int Order { get; }
        IEnumerator Export(ExportContext context);
    }
}
