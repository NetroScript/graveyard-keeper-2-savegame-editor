using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using LazyBearTechnology;
using UnityEngine;

namespace Gk2.AssetExporter
{
    public sealed class SpriteExporter : IDisposable
    {
        private readonly ExportContext context;
        private int cachedTextureId;
        private Color32[] cachedPixels;
        public readonly SortedDictionary<string, object> Images = new SortedDictionary<string, object>(StringComparer.Ordinal);
        public readonly SortedDictionary<string, object> Sprites = new SortedDictionary<string, object>(StringComparer.Ordinal);

        public SpriteExporter(ExportContext context) { this.context = context; }

        public string Sprite(string name)
        {
            if (string.IsNullOrEmpty(name)) return null;
            if (Sprites.ContainsKey(name)) return name;
            UnityEngine.Sprite sprite = null;
            try
            {
                sprite = EasySpritesCollection.Instance.GetSprite(name, null);
                if (sprite == null) throw new InvalidOperationException("Sprite is unavailable");
                var pixels = Read(sprite.texture);
                int width = Mathf.RoundToInt(sprite.rect.width), height = Mathf.RoundToInt(sprite.rect.height);
                // Atlas sprites may be trimmed, tightly packed, or rotated. Rasterize their
                // actual mesh/UVs onto the original canvas rather than cropping textureRect.
                var vertices = sprite.vertices;
                var uv = sprite.uv;
                var triangles = sprite.triangles;
                var positions = new Vector2[vertices.Length];
                for (int i = 0; i < vertices.Length; i++) positions[i] = vertices[i] * sprite.pixelsPerUnit + sprite.pivot;
                var output = new Color32[checked(width * height)];
                for (int i = 0; i < triangles.Length; i += 3)
                {
                    int ia = triangles[i], ib = triangles[i + 1], ic = triangles[i + 2];
                    var a = positions[ia]; var b = positions[ib]; var c = positions[ic];
                    float denominator = Cross(b - a, c - a);
                    if (Mathf.Abs(denominator) < 0.000001f) continue;
                    int minX = Math.Max(0, Mathf.FloorToInt(Math.Min(a.x, Math.Min(b.x, c.x))));
                    int minY = Math.Max(0, Mathf.FloorToInt(Math.Min(a.y, Math.Min(b.y, c.y))));
                    int maxX = Math.Min(width - 1, Mathf.CeilToInt(Math.Max(a.x, Math.Max(b.x, c.x))));
                    int maxY = Math.Min(height - 1, Mathf.CeilToInt(Math.Max(a.y, Math.Max(b.y, c.y))));
                    for (int y = minY; y <= maxY; y++) for (int x = minX; x <= maxX; x++)
                    {
                        var p = new Vector2(x + 0.5f, y + 0.5f) - a;
                        float wb = Cross(p, c - a) / denominator, wc = Cross(b - a, p) / denominator;
                        float wa = 1 - wb - wc;
                        if (wa < -0.00001f || wb < -0.00001f || wc < -0.00001f) continue;
                        var sample = uv[ia] * wa + uv[ib] * wb + uv[ic] * wc;
                        int sx = Mathf.Clamp(Mathf.FloorToInt(sample.x * sprite.texture.width), 0, sprite.texture.width - 1);
                        int sy = Mathf.Clamp(Mathf.FloorToInt(sample.y * sprite.texture.height), 0, sprite.texture.height - 1);
                        output[y * width + x] = pixels[sy * sprite.texture.width + sx];
                    }
                }
                var image = Save(width, height, output);
                Sprites[name] = new { image, width, height, pivot = new { x = sprite.pivot.x, y = sprite.pivot.y }, pixelsPerUnit = sprite.pixelsPerUnit };
                return name;
            }
            catch (Exception error)
            {
                Sprites[name] = new { image = (string)null, error = error.Message };
                context.Warn("sprite:" + name, error.Message);
                return name;
            }
            finally
            {
                // SpriteAtlas.GetSprite returns a clone. The atlas itself remains game-owned.
                if (sprite != null) UnityEngine.Object.Destroy(sprite);
            }
        }

        public string Crop(Texture texture, int x, int y, int width, int height)
        {
            if (width <= 0 || height <= 0 || x < 0 || y < 0 || x + width > texture.width || y + height > texture.height)
                throw new InvalidOperationException("Glyph rectangle is outside its atlas");
            var source = Read(texture);
            var pixels = new Color32[checked(width * height)];
            for (int row = 0; row < height; row++) Array.Copy(source, (y + row) * texture.width + x, pixels, row * width, width);
            return Save(width, height, pixels);
        }

        private Color32[] Read(Texture texture)
        {
            if (texture.GetInstanceID() == cachedTextureId && cachedPixels != null) return cachedPixels;
            var previous = RenderTexture.active;
            bool previousSrgb = GL.sRGBWrite;
            var target = RenderTexture.GetTemporary(texture.width, texture.height, 0, RenderTextureFormat.ARGB32, RenderTextureReadWrite.sRGB);
            Texture2D readable = null;
            try
            {
                // Unity decodes sRGB input on linear projects; re-encode for PNG output.
                GL.sRGBWrite = QualitySettings.activeColorSpace == ColorSpace.Linear;
                Graphics.Blit(texture, target);
                RenderTexture.active = target;
                readable = new Texture2D(texture.width, texture.height, TextureFormat.RGBA32, false);
                readable.ReadPixels(new Rect(0, 0, texture.width, texture.height), 0, 0);
                readable.Apply();
                cachedPixels = readable.GetPixels32();
                cachedTextureId = texture.GetInstanceID();
                return cachedPixels;
            }
            finally
            {
                GL.sRGBWrite = previousSrgb;
                RenderTexture.active = previous;
                RenderTexture.ReleaseTemporary(target);
                if (readable != null) UnityEngine.Object.Destroy(readable);
            }
        }

        private string Save(int width, int height, Color32[] pixels)
        {
            var texture = new Texture2D(width, height, TextureFormat.RGBA32, false);
            try
            {
                texture.SetPixels32(pixels);
                texture.Apply();
                byte[] bytes = texture.EncodeToPNG();
                string hash;
                using (var sha = SHA256.Create()) hash = BitConverter.ToString(sha.ComputeHash(bytes)).Replace("-", "").ToLowerInvariant();
                string path = "images/" + hash + ".png";
                if (!Images.ContainsKey(hash))
                {
                    Directory.CreateDirectory(Path.Combine(context.DirectoryPath, "images"));
                    File.WriteAllBytes(Path.Combine(context.DirectoryPath, path), bytes);
                    Images[hash] = new { path, width, height };
                }
                return hash;
            }
            finally { UnityEngine.Object.Destroy(texture); }
        }

        private static float Cross(Vector2 a, Vector2 b) { return a.x * b.y - a.y * b.x; }
        public void Dispose() { cachedPixels = null; cachedTextureId = 0; }
    }
}
