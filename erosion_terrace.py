"""
Эрозия, террасирование, океан и текстуры — Geometry Nodes (Blender 5.1)

Создаёт:
  1. Terrain  — Geometry Nodes: шум → эрозия → террасы
  2. Ocean    — Ocean Modifier: водная поверхность
  3. Material — процедурные текстуры: песок → трава → скала → снег

Запуск: Blender → Scripting → Open → Run Script
"""

import bpy
import math

# ══════════════════════════════════════════════════════════════
# КОНСТАНТЫ
# ══════════════════════════════════════════════════════════════
WATER_LEVEL = 0.3       # Z-уровень воды
OCEAN_SIZE  = 60.0      # размер плоскости океана
TERRAIN_SIZE = 50.0     # размер плоскости рельефа


# ══════════════════════════════════════════════════════════════
# 1. GEOMETRY NODES — рельеф + эрозия + террасы
# ══════════════════════════════════════════════════════════════
def create_terrain_nodes():
    name = "ErosionTerrace"
    if name in bpy.data.node_groups:
        bpy.data.node_groups.remove(bpy.data.node_groups[name])

    tree = bpy.data.node_groups.new(name, "GeometryNodeTree")
    lk = tree.links
    ifc = tree.interface

    # ── Интерфейс ──────────────────────────────────────────────
    ifc.new_socket("Geometry", in_out="INPUT", socket_type="NodeSocketGeometry")

    s_sub = ifc.new_socket("Subdivisions", in_out="INPUT", socket_type="NodeSocketInt")
    s_sub.default_value = 6;  s_sub.min_value = 1;  s_sub.max_value = 50

    s_sc = ifc.new_socket("Noise Scale", in_out="INPUT", socket_type="NodeSocketFloat")
    s_sc.default_value = 0.5;  s_sc.min_value = 0.01;  s_sc.max_value = 20.0

    s_ht = ifc.new_socket("Terrain Height", in_out="INPUT", socket_type="NodeSocketFloat")
    s_ht.default_value = 5.0;  s_ht.min_value = 0.0;  s_ht.max_value = 20.0

    s_er = ifc.new_socket("Erosion Strength", in_out="INPUT", socket_type="NodeSocketFloat")
    s_er.default_value = 0.5;  s_er.min_value = 0.0;  s_er.max_value = 5.0

    s_tl = ifc.new_socket("Terrace Levels", in_out="INPUT", socket_type="NodeSocketInt")
    s_tl.default_value = 8;  s_tl.min_value = 2;  s_tl.max_value = 64

    s_ts = ifc.new_socket("Terrace Smoothness", in_out="INPUT", socket_type="NodeSocketFloat")
    s_ts.default_value = 0.15;  s_ts.min_value = 0.0;  s_ts.max_value = 1.0

    ifc.new_socket("Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry")

    gi = tree.nodes.new("NodeGroupInput");   gi.location = (-1400, 0)
    go = tree.nodes.new("NodeGroupOutput");  go.location = (1800, 0)

    # ── 1. Subdivide ───────────────────────────────────────────
    n_sub = tree.nodes.new("GeometryNodeSubdivideMesh")
    n_sub.location = (-1100, 0)
    lk.new(gi.outputs["Geometry"],     n_sub.inputs["Mesh"])
    lk.new(gi.outputs["Subdivisions"], n_sub.inputs["Level"])

    # ── 2. Рельеф: Noise → Offset Z ───────────────────────────
    n_pos1  = tree.nodes.new("GeometryNodeInputPosition")
    n_pos1.location = (-1100, -300)

    n_noise = tree.nodes.new("ShaderNodeTexNoise")
    n_noise.location = (-800, -300)
    n_noise.inputs["Detail"].default_value = 8.0
    n_noise.inputs["Roughness"].default_value = 0.5

    n_sub05 = tree.nodes.new("ShaderNodeMath")
    n_sub05.operation = "SUBTRACT"
    n_sub05.inputs[1].default_value = 0.5
    n_sub05.location = (-600, -300)

    n_mul2 = tree.nodes.new("ShaderNodeMath")
    n_mul2.operation = "MULTIPLY"
    n_mul2.inputs[1].default_value = 2.0
    n_mul2.location = (-400, -300)

    n_mulh = tree.nodes.new("ShaderNodeMath")
    n_mulh.operation = "MULTIPLY"
    n_mulh.location = (-200, -300)

    n_co1 = tree.nodes.new("ShaderNodeCombineXYZ")
    n_co1.location = (0, -300)

    n_sp1 = tree.nodes.new("GeometryNodeSetPosition")
    n_sp1.location = (200, 0)

    lk.new(n_sub.outputs["Mesh"],        n_sp1.inputs["Geometry"])
    lk.new(n_pos1.outputs["Position"],   n_noise.inputs["Vector"])
    lk.new(gi.outputs["Noise Scale"],    n_noise.inputs["Scale"])
    lk.new(n_noise.outputs["Fac"],       n_sub05.inputs[0])
    lk.new(n_sub05.outputs[0],           n_mul2.inputs[0])
    lk.new(n_mul2.outputs[0],            n_mulh.inputs[0])
    lk.new(gi.outputs["Terrain Height"], n_mulh.inputs[1])
    lk.new(n_mulh.outputs[0],            n_co1.inputs["Z"])
    lk.new(n_co1.outputs["Vector"],      n_sp1.inputs["Offset"])

    # ── 3. Эрозия ──────────────────────────────────────────────
    n_pos2 = tree.nodes.new("GeometryNodeInputPosition")
    n_pos2.location = (200, -400)

    n_sep2 = tree.nodes.new("ShaderNodeSeparateXYZ")
    n_sep2.location = (400, -400)

    n_norm = tree.nodes.new("GeometryNodeInputNormal")
    n_norm.location = (200, -700)

    n_sepn = tree.nodes.new("ShaderNodeSeparateXYZ")
    n_sepn.location = (400, -700)

    n_stp = tree.nodes.new("ShaderNodeMath")
    n_stp.operation = "SUBTRACT"
    n_stp.inputs[0].default_value = 1.0
    n_stp.location = (600, -700)

    n_em = tree.nodes.new("ShaderNodeMath")
    n_em.operation = "MULTIPLY"
    n_em.location = (800, -700)

    n_ez = tree.nodes.new("ShaderNodeMath")
    n_ez.operation = "SUBTRACT"
    n_ez.location = (1000, -400)

    lk.new(n_pos2.outputs["Position"],     n_sep2.inputs["Vector"])
    lk.new(n_norm.outputs["Normal"],       n_sepn.inputs["Vector"])
    lk.new(n_sepn.outputs["Z"],            n_stp.inputs[1])
    lk.new(n_stp.outputs[0],              n_em.inputs[0])
    lk.new(gi.outputs["Erosion Strength"], n_em.inputs[1])
    lk.new(n_sep2.outputs["Z"],            n_ez.inputs[0])
    lk.new(n_em.outputs[0],                n_ez.inputs[1])

    # ── 4. Террасирование ──────────────────────────────────────
    n_mlv = tree.nodes.new("ShaderNodeMath")
    n_mlv.operation = "MULTIPLY"
    n_mlv.location = (1100, -400)

    n_snap = tree.nodes.new("ShaderNodeMath")
    n_snap.operation = "SNAP"
    n_snap.inputs[1].default_value = 1.0
    n_snap.location = (1200, -400)

    n_dv = tree.nodes.new("ShaderNodeMath")
    n_dv.operation = "DIVIDE"
    n_dv.location = (1300, -400)

    lk.new(n_ez.outputs[0],              n_mlv.inputs[0])
    lk.new(gi.outputs["Terrace Levels"],  n_mlv.inputs[1])
    lk.new(n_mlv.outputs[0],             n_snap.inputs[0])
    lk.new(n_snap.outputs[0],            n_dv.inputs[0])
    lk.new(gi.outputs["Terrace Levels"],  n_dv.inputs[1])

    # ── 5. Смешивание ──────────────────────────────────────────
    n_inv = tree.nodes.new("ShaderNodeMath")
    n_inv.operation = "SUBTRACT"
    n_inv.inputs[0].default_value = 1.0
    n_inv.location = (1100, -200)

    n_mt = tree.nodes.new("ShaderNodeMath")
    n_mt.operation = "MULTIPLY"
    n_mt.location = (1300, -200)

    n_mo = tree.nodes.new("ShaderNodeMath")
    n_mo.operation = "MULTIPLY"
    n_mo.location = (1300, -600)

    n_fz = tree.nodes.new("ShaderNodeMath")
    n_fz.operation = "ADD"
    n_fz.location = (1500, -400)

    lk.new(gi.outputs["Terrace Smoothness"], n_inv.inputs[1])
    lk.new(n_inv.outputs[0],                  n_mt.inputs[0])
    lk.new(n_dv.outputs[0],                   n_mt.inputs[1])
    lk.new(gi.outputs["Terrace Smoothness"],  n_mo.inputs[0])
    lk.new(n_ez.outputs[0],                   n_mo.inputs[1])
    lk.new(n_mt.outputs[0],                   n_fz.inputs[0])
    lk.new(n_mo.outputs[0],                   n_fz.inputs[1])

    # ── 6. Финальная позиция ───────────────────────────────────
    n_cf = tree.nodes.new("ShaderNodeCombineXYZ")
    n_cf.location = (1500, 0)

    n_sp2 = tree.nodes.new("GeometryNodeSetPosition")
    n_sp2.location = (1600, 0)

    lk.new(n_sep2.outputs["X"],          n_cf.inputs["X"])
    lk.new(n_sep2.outputs["Y"],          n_cf.inputs["Y"])
    lk.new(n_fz.outputs[0],             n_cf.inputs["Z"])
    lk.new(n_sp1.outputs["Geometry"],    n_sp2.inputs["Geometry"])
    lk.new(n_cf.outputs["Vector"],       n_sp2.inputs["Position"])
    lk.new(n_sp2.outputs["Geometry"],    go.inputs["Geometry"])

    return tree


# ══════════════════════════════════════════════════════════════
# 2. OCEAN MODIFIER — водная поверхность
# ══════════════════════════════════════════════════════════════
def create_ocean():
    # Удалить старый океан
    old = bpy.data.objects.get("Ocean")
    if old:
        bpy.data.objects.remove(old, do_unlink=True)

    bpy.ops.mesh.primitive_plane_add(size=OCEAN_SIZE, location=(0, 0, WATER_LEVEL))
    ocean = bpy.context.object
    ocean.name = "Ocean"
    ocean.scale = (1, 1, 1)

    # Ocean Modifier
    mod = ocean.modifiers.new("Ocean", "OCEAN")
    mod.geometry_mode = "GENERATE"
    mod.spatial_size = 128
    mod.resolution = 7
    mod.wind_velocity = 5.0
    mod.damping = 0.5
    mod.wave_scale = 0.3
    mod.wave_scale_min = 0.0
    mod.choppiness = 0.5
    mod.foam_coverage = 0.1
    mod.time = 1.0
    mod.random_seed = 0

    # Материал воды
    ocean.data.materials.append(create_water_material())

    print(f"[OK] Ocean created at Z={WATER_LEVEL}")
    return ocean


# ══════════════════════════════════════════════════════════════
# 3. МАТЕРИАЛЫ
# ══════════════════════════════════════════════════════════════
def create_water_material():
    """Прозрачная вода с Fresnel."""
    name = "M_Ocean"
    if name in bpy.data.materials:
        return bpy.data.materials[name]

    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    mat.use_backface_culling = True
    if hasattr(mat, "blend_method"):
        mat.blend_method = "BLEND"
    if hasattr(mat, "shadow_method"):
        mat.shadow_method = "HASHED"

    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    out = nodes.get("Material Output")
    bsdf = nodes.get("Principled BSDF")

    # Параметры BSDF
    bsdf.inputs["Base Color"].default_value = (0.01, 0.08, 0.18, 1.0)
    bsdf.inputs["Roughness"].default_value = 0.05
    bsdf.inputs["Alpha"].default_value = 0.7
    bsdf.inputs["IOR"].default_value = 1.33

    # Noise для лёгкой ряби
    n_noise = nodes.new("ShaderNodeTexNoise")
    n_noise.inputs["Scale"].default_value = 40.0
    n_noise.inputs["Detail"].default_value = 4.0
    n_noise.inputs["Roughness"].default_value = 0.6
    n_noise.location = (-400, -200)

    # Color Ramp: тёмная вода ↔ светлая пена
    n_cr = nodes.new("ShaderNodeValToRGB")
    n_cr.location = (-200, -200)
    # Пена на гребнях
    elems = n_cr.color_ramp.elements
    elems[0].position = 0.4;  elems[0].color = (0.01, 0.08, 0.18, 1.0)
    elems[1].position = 0.85; elems[1].color = (0.12, 0.25, 0.35, 1.0)

    links.new(n_noise.outputs["Fac"], n_cr.inputs["Fac"])
    links.new(n_cr.outputs["Color"],  bsdf.inputs["Base Color"])

    # Fresnel → Mix с Alpha
    n_fres = nodes.new("ShaderNodeFresnel")
    n_fres.inputs["IOR"].default_value = 1.33
    n_fres.location = (-400, 0)

    links.new(n_fres.outputs[0], bsdf.inputs["Alpha"])

    return mat


def create_terrain_material():
    """Процедурный ландшафт: песок → трава → скала → снег."""
    name = "M_Terrain"
    if name in bpy.data.materials:
        bpy.data.materials.remove(bpy.data.materials[name])

    mat = bpy.data.materials.new(name)
    mat.use_nodes = True

    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    out = nodes.get("Material Output")
    bsdf = nodes.get("Principled BSDF")
    bsdf.location = (200, 0)
    out.location = (600, 0)

    # ── Позиция (объектные координаты) ─────────────────────────
    n_geo = nodes.new("ShaderNodeNewGeometry")
    n_geo.location = (-1000, 300)

    n_sep = nodes.new("ShaderNodeSeparateXYZ")
    n_sep.location = (-800, 300)
    links.new(n_geo.outputs["Position"], n_sep.inputs["Vector"])

    # ── Крутизна = 1 − Nz ─────────────────────────────────────
    n_sep_n = nodes.new("ShaderNodeSeparateXYZ")
    n_sep_n.location = (-800, 50)
    links.new(n_geo.outputs["Normal"], n_sep_n.inputs["Vector"])

    n_stp = nodes.new("ShaderNodeMath")
    n_stp.operation = "SUBTRACT"
    n_stp.inputs[0].default_value = 1.0
    n_stp.location = (-600, 50)
    links.new(n_sep_n.outputs["Z"], n_stp.inputs[1])

    # ── Ремаппинг высоты: (Z − WaterLevel) / Height → 0..1 ────
    n_zsub = nodes.new("ShaderNodeMath")
    n_zsub.operation = "SUBTRACT"
    n_zsub.inputs[1].default_value = WATER_LEVEL
    n_zsub.location = (-600, 300)
    links.new(n_sep.outputs["Z"], n_zsub.inputs[0])

    n_zdiv = nodes.new("ShaderNodeMath")
    n_zdiv.operation = "DIVIDE"
    n_zdiv.inputs[1].default_value = 5.0   # Terrain Height
    n_zdiv.location = (-500, 300)
    links.new(n_zsub.outputs[0], n_zdiv.inputs[0])

    # Clamp 0..1
    n_zclamp = nodes.new("ShaderNodeMath")
    n_zclamp.operation = "MULTIPLY_ADD"
    n_zclamp.inputs[1].default_value = 1.0
    n_zclamp.inputs[2].default_value = 0.0
    n_zclamp.use_clamp = True
    n_zclamp.location = (-400, 300)
    links.new(n_zdiv.outputs[0], n_zclamp.inputs[0])

    # ── Высотный ColorRamp: песок → трава → скала → снег ──────
    n_hcr = nodes.new("ShaderNodeValToRGB")
    n_hcr.location = (-200, 300)
    h = n_hcr.color_ramp.elements
    h[0].position = 0.00; h[0].color = (0.95, 0.90, 0.65, 1.0)   # песок
    e1 = h.new(0.10); e1.color = (0.95, 0.90, 0.65, 1.0)         # песок
    e2 = h.new(0.18); e2.color = (0.22, 0.50, 0.10, 1.0)         # трава
    e3 = h.new(0.55); e3.color = (0.15, 0.38, 0.08, 1.0)         # тёмная трава
    e4 = h.new(0.65); e4.color = (0.40, 0.32, 0.22, 1.0)         # переход
    e5 = h.new(0.75); e5.color = (0.50, 0.42, 0.32, 1.0)         # скала
    e6 = h.new(0.88); e6.color = (0.95, 0.97, 1.00, 1.0)         # снег
    h[1].position = 1.00; h[1].color = (0.95, 0.97, 1.00, 1.0)   # снег
    links.new(n_zclamp.outputs[0], n_hcr.inputs["Fac"])

    # ── Скальный оверлей по крутизне ───────────────────────────
    # Скала только на крутых склонах (steepness > 0.55)
    n_smul = nodes.new("ShaderNodeMath")
    n_smul.operation = "SUBTRACT"
    n_smul.inputs[1].default_value = 0.55
    n_smul.location = (-400, 50)
    links.new(n_stp.outputs[0], n_smul.inputs[0])

    # (steepness − 0.55) × 4.0 → скала от 0.55 до 0.8
    n_smul2 = nodes.new("ShaderNodeMath")
    n_smul2.operation = "MULTIPLY"
    n_smul2.inputs[1].default_value = 4.0
    n_smul2.location = (-300, 50)
    links.new(n_smul.outputs[0], n_smul2.inputs[0])

    n_sclamp = nodes.new("ShaderNodeMath")
    n_sclamp.operation = "MULTIPLY_ADD"
    n_sclamp.inputs[1].default_value = 1.0
    n_sclamp.inputs[2].default_value = 0.0
    n_sclamp.use_clamp = True
    n_sclamp.location = (-200, 50)
    links.new(n_smul2.outputs[0], n_sclamp.inputs[0])

    # Mix height_color ↔ rock_color (MixRGB)
    n_rmix = nodes.new("ShaderNodeMixRGB")
    n_rmix.blend_type = "MIX"
    n_rmix.location = (0, 200)
    links.new(n_sclamp.outputs[0],       n_rmix.inputs["Factor"])
    links.new(n_hcr.outputs["Color"],    n_rmix.inputs["Color1"])
    n_rmix.inputs["Color2"].default_value = (0.42, 0.35, 0.28, 1.0)

    # ── Noise текстура для вариации ────────────────────────────
    n_noise1 = nodes.new("ShaderNodeTexNoise")
    n_noise1.inputs["Scale"].default_value = 15.0
    n_noise1.inputs["Detail"].default_value = 8.0
    n_noise1.inputs["Roughness"].default_value = 0.5
    n_noise1.location = (-600, -200)
    links.new(n_geo.outputs["Position"], n_noise1.inputs["Vector"])

    n_ncr = nodes.new("ShaderNodeValToRGB")
    n_ncr.location = (-400, -200)
    ne = n_ncr.color_ramp.elements
    ne[0].position = 0.0; ne[0].color = (0.85, 0.80, 0.60, 1.0)
    ne[1].position = 1.0; ne[1].color = (0.30, 0.45, 0.15, 1.0)
    links.new(n_noise1.outputs["Fac"], n_ncr.inputs["Fac"])

    # Mix base + noise variation (20%)
    n_vmix = nodes.new("ShaderNodeMixRGB")
    n_vmix.blend_type = "MIX"
    n_vmix.inputs["Factor"].default_value = 0.20
    n_vmix.location = (200, 100)
    links.new(n_rmix.outputs["Color"],    n_vmix.inputs["Color1"])
    links.new(n_ncr.outputs["Color"],     n_vmix.inputs["Color2"])
    links.new(n_vmix.outputs["Color"],    bsdf.inputs["Base Color"])

    # ── Bump (рельефность поверхности) ─────────────────────────
    n_bnoise = nodes.new("ShaderNodeTexNoise")
    n_bnoise.inputs["Scale"].default_value = 50.0
    n_bnoise.inputs["Detail"].default_value = 10.0
    n_bnoise.inputs["Roughness"].default_value = 0.6
    n_bnoise.location = (-200, -450)
    links.new(n_geo.outputs["Position"], n_bnoise.inputs["Vector"])

    n_bump = nodes.new("ShaderNodeBump")
    n_bump.inputs["Strength"].default_value = 0.4
    n_bump.inputs["Distance"].default_value = 0.05
    n_bump.location = (0, -450)
    links.new(n_bnoise.outputs["Fac"], n_bump.inputs["Height"])
    links.new(n_bump.outputs["Normal"], bsdf.inputs["Normal"])

    # ── Roughness по крутизне ──────────────────────────────────
    n_rfun = nodes.new("ShaderNodeMath")
    n_rfun.operation = "MULTIPLY_ADD"
    n_rfun.inputs[1].default_value = 0.5
    n_rfun.inputs[2].default_value = 0.3
    n_rfun.use_clamp = True
    n_rfun.location = (0, -100)
    links.new(n_sclamp.outputs[0], n_rfun.inputs[0])
    links.new(n_rfun.outputs[0], bsdf.inputs["Roughness"])

    return mat


# ══════════════════════════════════════════════════════════════
# MAIN
# ══════════════════════════════════════════════════════════════
def main():
    # ── Terrain ────────────────────────────────────────────────
    tree = create_terrain_nodes()

    # Найти или создать плоскость рельефа
    terrain = bpy.data.objects.get("Terrain")
    if not terrain:
        bpy.ops.mesh.primitive_plane_add(size=TERRAIN_SIZE)
        terrain = bpy.context.object
        terrain.name = "Terrain"

    # Удалить старый модификатор
    for m in list(terrain.modifiers):
        if m.type == "NODES" and "Erosion" in m.name:
            terrain.modifiers.remove(m)

    mod = terrain.modifiers.new("ErosionTerrace", "NODES")
    mod.node_group = tree

    # Материал рельефа
    terrain_mat = create_terrain_material()
    if len(terrain.data.materials) == 0:
        terrain.data.materials.append(terrain_mat)
    else:
        terrain.data.materials[0] = terrain_mat

    # ── Ocean ──────────────────────────────────────────────────
    ocean = create_ocean()

    # ── Камера и свет ──────────────────────────────────────────
    cam = bpy.data.objects.get("Camera")
    if not cam:
        bpy.ops.object.camera_add(location=(0, -30, 20))
        cam = bpy.context.object
        cam.name = "Camera"
        cam.rotation_euler = (math.radians(55), 0, 0)
        bpy.context.scene.camera = cam

    sun = bpy.data.objects.get("Sun")
    if not sun:
        bpy.ops.object.light_add(type="SUN", location=(5, -3, 10))
        sun = bpy.context.object
        sun.name = "Sun"
        sun.data.energy = 5.0
        sun.rotation_euler = (math.radians(45), math.radians(15), math.radians(30))

    # EEVEE / Workbench → Cycles для лучшего результата
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 128

    print(f"[OK] Terrain + Ocean + Materials created")
    print(f"     Water level = {WATER_LEVEL}")
    print(f"     Terrain size = {TERRAIN_SIZE}, Ocean size = {OCEAN_SIZE}")


main()
