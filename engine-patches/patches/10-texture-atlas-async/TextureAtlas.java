// АГРЕССИВНЫЙ патч: Полная параллельная загрузка ВСЕХ текстур
// ЦЕЛЬ: Сократить время загрузки текстур на 70%+
// РИСК: МАКСИМАЛЬНЫЙ - полная реструктуризация TextureAtlas
// ЕСЛИ СЛОМАЕТСЯ - ЧИНИМ ПАТЧ, НЕ ОТКАТЫВАЕМ!

package net.minecraft.client.renderer.texture;

import com.mojang.blaze3d.platform.TextureUtil;
import com.mojang.blaze3d.systems.RenderSystem;
import com.mojang.logging.LogUtils;
import java.io.IOException;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Map.Entry;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ForkJoinPool;
import java.util.stream.Collectors;
import net.minecraft.CrashReport;
import net.minecraft.CrashReportCategory;
import net.minecraft.ReportedException;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.packs.resources.ResourceManager;
import net.minecraft.world.inventory.InventoryMenu;
import net.minecraftforge.api.distmarker.Dist;
import net.minecraftforge.api.distmarker.OnlyIn;
import org.slf4j.Logger;

@OnlyIn(Dist.CLIENT)
public class TextureAtlas extends AbstractTexture implements Dumpable, Tickable {
   private static final Logger LOGGER = LogUtils.getLogger();
   
   // АГРЕССИВНО: Используем общий ForkJoinPool для максимального параллелизма
   private static final ForkJoinPool TEXTURE_POOL = ForkJoinPool.commonPool();
   
   public static final ResourceLocation LOCATION_BLOCKS = InventoryMenu.BLOCK_ATLAS;
   public static final ResourceLocation LOCATION_PARTICLES = new ResourceLocation("textures/atlas/particles.png");
   
   private List<SpriteContents> sprites = List.of();
   private List<TextureAtlasSprite.Ticker> animatedTextures = List.of();
   private Map<ResourceLocation, TextureAtlasSprite> texturesByName = Map.of();
   private final ResourceLocation location;
   private final int maxSupportedTextureSize;
   private int width;
   private int height;
   private int mipLevel;

   public TextureAtlas(ResourceLocation location) {
      this.location = location;
      this.maxSupportedTextureSize = RenderSystem.maxSupportedTextureSize();
   }

   public void load(ResourceManager rm) {
   }

   // АГРЕССИВНО: ПОЛНАЯ параллельная загрузка с замером времени
   public void upload(SpriteLoader.Preparations preparations) {
      long startTime = System.nanoTime();
      
      LOGGER.info("[АГРЕССИВНО] Загрузка атласа {} ({}x{}, mip={})", 
         this.location, preparations.width(), preparations.height(), preparations.mipLevel());
      
      TextureUtil.prepareImage(this.getId(), preparations.mipLevel(), preparations.width(), preparations.height());
      this.width = preparations.width();
      this.height = preparations.height();
      this.mipLevel = preparations.mipLevel();
      this.clearTextureData();
      this.texturesByName = Map.copyOf(preparations.regions());
      
      List<CompletableFuture<Void>> futures = new ArrayList<>();
      List<SpriteContents> spriteList = new ArrayList<>();
      List<TextureAtlasSprite.Ticker> tickerList = new ArrayList<>();

      // АГРЕССИВНО: Параллельная обработка ВСЕХ спрайтов через ForkJoinPool
      preparations.regions().values().parallelStream().forEach(sprite -> {
         try {
            sprite.uploadFirstFrame();
            synchronized(spriteList) {
               spriteList.add(sprite.contents());
               TextureAtlasSprite.Ticker ticker = sprite.createTicker();
               if (ticker != null) {
                  synchronized(tickerList) {
                     tickerList.add(ticker);
                  }
               }
            }
         } catch (Throwable throwable) {
            CrashReport crashreport = CrashReport.forThrowable(throwable, "Stitching texture atlas");
            CrashReportCategory crashreportcategory = crashreport.addCategory("Texture being stitched together");
            crashreportcategory.setDetail("Atlas path", this.location);
            crashreportcategory.setDetail("Sprite", sprite);
            throw new ReportedException(crashreport);
         }
      });
      
      this.sprites = List.copyOf(spriteList);
      this.animatedTextures = List.copyOf(tickerList);
      
      long endTime = System.nanoTime();
      long durationMs = (endTime - startTime) / 1_000_000;
      
      LOGGER.info("[АГРЕССИВНО] Атлас {} загружен: {} спрайтов за {}мс", this.location, this.sprites.size(), durationMs);
   }

   public void dumpContents(ResourceLocation location, Path path) throws IOException {
      String s = location.toDebugFileName();
      TextureUtil.writeAsPNG(path, s, this.getId(), this.mipLevel, this.width, this.height);
      dumpSpriteNames(path, s, this.texturesByName);
   }

   private static void dumpSpriteNames(Path path, String name, Map<ResourceLocation, TextureAtlasSprite> textures) {
      Path filePath = path.resolve(name + ".txt");

      try (Writer writer = Files.newBufferedWriter(filePath)) {
         for(Map.Entry<ResourceLocation, TextureAtlasSprite> entry : textures.entrySet().stream().sorted(Entry.comparingByKey()).collect(Collectors.toList())) {
            TextureAtlasSprite sprite = entry.getValue();
            writer.write(String.format(Locale.ROOT, "%s\tx=%d\ty=%d\tw=%d\th=%d%n", entry.getKey(), sprite.getX(), sprite.getY(), sprite.contents().width(), sprite.contents().height()));
         }
      } catch (IOException ioexception) {
         LOGGER.warn("Failed to write file {}", path, ioexception);
      }
   }
}
