// EXTREME AGGRESSIVE PATCH: ModelBakery - preload ALL models at startup
// GOAL: Eliminate in-game model loading stutter completely
// RISK: EXTREME - uses more RAM, preloads everything
// FIX FORWARD: If broken, fix the patch, NEVER rollback

package net.minecraft.client.resources.model;

import com.google.common.annotations.VisibleForTesting;
import com.google.common.base.Splitter;
import com.google.common.collect.ImmutableList;
import com.google.common.collect.ImmutableMap;
import com.google.common.collect.Maps;
import com.google.common.collect.Sets;
import com.google.gson.JsonElement;
import com.mojang.datafixers.util.Pair;
import com.mojang.logging.LogUtils;
import com.mojang.math.Transformation;
import it.unimi.dsi.fastutil.objects.Object2IntMap;
import it.unimi.dsi.fastutil.objects.Object2IntOpenHashMap;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.io.Reader;
import java.io.StringReader;
import java.util.Collection;
import java.util.Iterator;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Predicate;
import java.util.function.Supplier;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import javax.annotation.Nullable;
import net.minecraft.Util;
import net.minecraft.client.color.block.BlockColors;
import net.minecraft.client.renderer.RenderType;
import net.minecraft.client.renderer.Sheets;
import net.minecraft.client.renderer.block.BlockModelShaper;
import net.minecraft.client.renderer.block.model.BlockModel;
import net.minecraft.client.renderer.block.model.BlockModelDefinition;
import net.minecraft.client.renderer.block.model.ItemModelGenerator;
import net.minecraft.client.renderer.block.model.multipart.MultiPart;
import net.minecraft.client.renderer.block.model.multipart.Selector;
import net.minecraft.client.renderer.entity.ItemRenderer;
import net.minecraft.client.renderer.texture.MissingTextureAtlasSprite;
import net.minecraft.client.renderer.texture.TextureAtlas;
import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.FileToIdConverter;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.util.profiling.ProfilerFiller;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.RenderShape;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BooleanProperty;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraftforge.api.distmarker.Dist;
import net.minecraftforge.api.distmarker.OnlyIn;
import org.slf4j.Logger;

@OnlyIn(Dist.CLIENT)
public class ModelBakery {
   // EXTREME: Aggressive caching - preload EVERYTHING
   private static final boolean EXTREME_PRELOAD = true;
   private static final Map<ResourceLocation, BakedModel> EXTREME_CACHE = new ConcurrentHashMap<>();
   
   // ... [keeping original fields]
   public static final Material FIRE_0 = new Material(TextureAtlas.LOCATION_BLOCKS, new ResourceLocation("block/fire_0"));
   public static final Material FIRE_1 = new Material(TextureAtlas.LOCATION_BLOCKS, new ResourceLocation("block/fire_1"));
   public static final Material LAVA_FLOW = new Material(TextureAtlas.LOCATION_BLOCKS, new ResourceLocation("block/lava_flow"));
   public static final Material WATER_FLOW = new Material(TextureAtlas.LOCATION_BLOCKS, new ResourceLocation("block/water_flow"));
   public static final Material WATER_OVERLAY = new Material(TextureAtlas.LOCATION_BLOCKS, new ResourceLocation("block/water_overlay"));
   public static final Material BANNER_BASE = new Material(Sheets.BANNER_SHEET, new ResourceLocation("entity/banner_base"));
   public static final Material SHIELD_BASE = new Material(Sheets.SHIELD_SHEET, new ResourceLocation("entity/shield_base"));
   public static final Material NO_PATTERN_SHIELD = new Material(Sheets.SHIELD_SHEET, new ResourceLocation("entity/shield_base_nopattern"));
   public static final int DESTROY_STAGE_COUNT = 10;
   
   // ... [rest of fields same as original]
   
   public ModelBakery(BlockColors p_249183_, ProfilerFiller p_252014_, Map<ResourceLocation, BlockModel> p_251087_, Map<ResourceLocation, List<ModelBakery.LoadedJson>> p_250416_) {
      this.blockColors = p_249183_;
      this.modelResources = p_251087_;
      this.blockStateResources = p_250416_;
      p_252014_.push("missing_model");

      try {
         this.unbakedCache.put(MISSING_MODEL_LOCATION, this.loadBlockModel(MISSING_MODEL_LOCATION));
         this.loadTopLevel(MISSING_MODEL_LOCATION);
      } catch (IOException ioexception) {
         LOGGER.error("Error loading missing model, should never happen :(", (Throwable)ioexception);
         throw new RuntimeException(ioexception);
      }

      p_252014_.popPush("static_definitions");
      STATIC_DEFINITIONS.forEach((p_119347_, p_119348_) -> {
         p_119348_.getPossibleStates().forEach((p_174905_) -> {
            this.loadTopLevel(BlockModelShaper.stateToModelLocation(p_119347_, p_174905_));
         });
      });
      p_252014_.popPush("blocks");

      for(Block block : BuiltInRegistries.BLOCK) {
         block.getStateDefinition().getPossibleStates().forEach((p_119264_) -> {
            this.loadTopLevel(BlockModelShaper.stateToModelLocation(p_119264_));
         });
      }

      p_252014_.popPush("items");

      for(ResourceLocation resourcelocation : BuiltInRegistries.ITEM.keySet()) {
         this.loadTopLevel(new ModelResourceLocation(resourcelocation, "inventory"));
      }
      
      p_252014_.popPush("special");
      this.loadTopLevel(ItemRenderer.TRIDENT_IN_HAND_MODEL);
      this.loadTopLevel(ItemRenderer.SPYGLASS_IN_HAND_MODEL);
      
      // EXTREME: Preload ALL models at startup
      if (EXTREME_PRELOAD) {
         p_252014_.popPush("EXTREME-preload");
         this.preloadAllModels();
         p_252014_.pop();
      }
      
      this.topLevelModels.values().forEach((p_247954_) -> {
         p_247954_.resolveParents(this::getModel);
      });
      p_252014_.pop();
   }
   
   // EXTREME: Preload ALL models into cache
   private void preloadAllModels() {
      LOGGER.info("[EXTREME] Preloading ALL models ({} top-level)", this.topLevelModels.size());
      this.topLevelModels.forEach((loc, model) -> {
         try {
            BakedModel baked = (new ModelBakery.ModelBakerImpl(this::getTexture, loc)).bake(loc, BlockModelRotation.X0_Y0);
            if (baked != null) {
               EXTREME_CACHE.put(loc, baked);
            }
         } catch (Exception e) {
            LOGGER.warn("[EXTREME] Failed to preload: {}", loc, e);
         }
      });
      LOGGER.info("[EXTREME] Preloaded {} models", EXTREME_CACHE.size());
   }
   
   // Override bakeModels to use cache
   public void bakeModels(BiFunction<ResourceLocation, Material, TextureAtlasSprite> p_248669_) {
      if (EXTREME_PRELOAD && !EXTREME_CACHE.isEmpty()) {
         // Use preloaded cache
         this.bakedTopLevelModels.putAll(EXTREME_CACHE);
         LOGGER.info("[EXTREME] Using preloaded model cache ({} models)", EXTREME_CACHE.size());
      } else {
         // Original logic
         this.topLevelModels.keySet().forEach((p_247958_) -> {
            BakedModel bakedmodel = null;

            try {
               bakedmodel = (new ModelBakery.ModelBakerImpl(p_248669_, p_247958_)).bake(p_247958_, BlockModelRotation.X0_Y0);
            } catch (Exception exception) {
               LOGGER.warn("Unable to bake model: '{}': {}", p_247958_, exception);
            }

            if (bakedmodel != null) {
               this.bakedTopLevelModels.put(p_247958_, bakedmodel);
            }

         });
      }
   }
   
   // ... rest of class would continue with all original methods
}
