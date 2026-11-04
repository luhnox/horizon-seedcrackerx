import com.seedfinding.mccore.rand.ChunkRand;
import com.seedfinding.mccore.rand.seed.*;
import com.seedfinding.mccore.util.pos.CPos;
import com.seedfinding.mccore.version.MCVersion;
import com.seedfinding.mcfeature.decorator.DesertWell;
import com.seedfinding.mcfeature.decorator.EndGateway;
import com.seedfinding.mcfeature.structure.*;
import com.seedfinding.mcseed.rand.JRand;
import kaptainwutax.seedcrackerX.structures.TrialChambers;

import java.util.*;
import java.util.function.Function;

/** Prints what the Java library answers, for the Rust port to be checked against line by line. */
public class Gen {
    static final MCVersion[] VERSIONS = {MCVersion.v1_12_2, MCVersion.v1_13, MCVersion.v1_13_2, MCVersion.v1_16_5, MCVersion.v1_20_6, MCVersion.v1_21_3};

    static String pos(CPos p) {
        return p == null ? "none" : p.getX() + "," + p.getZ();
    }

    public static void main(String[] args) {
        Random r = new Random(20261104L);
        List<Long> seeds = new ArrayList<>();
        seeds.add(0L);
        seeds.add(1L);
        seeds.add(-4172144997902289642L);
        seeds.add(0x00009a3b77c155d2L);
        for (int i = 0; i < 30; i++) seeds.add(r.nextLong() & 0xFFFFFFFFFFFFL);
        List<Long> worldSeeds = new ArrayList<>();
        for (int i = 0; i < 12; i++) worldSeeds.add(r.nextLong());
        worldSeeds.add(0L);
        worldSeeds.add(-1L);
        ChunkRand rand = new ChunkRand();

        // Structures: where each is in a few regions, and whether the data there (and next to it) starts for the seed.
        Map<String, Function<MCVersion, RegionStructure<?, ?>>> classes = new LinkedHashMap<>();
        classes.put("buried_treasure", BuriedTreasure::new);
        classes.put("desert_pyramid", DesertPyramid::new);
        classes.put("end_city", EndCity::new);
        classes.put("igloo", Igloo::new);
        classes.put("jungle_pyramid", JunglePyramid::new);
        classes.put("monument", Monument::new);
        classes.put("pillager_outpost", PillagerOutpost::new);
        classes.put("shipwreck", Shipwreck::new);
        classes.put("swamp_hut", SwampHut::new);
        classes.put("village", Village::new);
        classes.put("trial_chambers", TrialChambers::new);
        for (MCVersion v : VERSIONS) {
            for (Map.Entry<String, Function<MCVersion, RegionStructure<?, ?>>> e : classes.entrySet()) {
                RegionStructure<?, ?> s;
                try {
                    s = e.getValue().apply(v);
                    s.getSpacing();
                } catch (Throwable t) {
                    System.out.println("CFG " + e.getKey() + " " + v + " none");
                    continue;
                }
                System.out.println("CFG " + e.getKey() + " " + v + " " + s.getSpacing() + " " + s.getSeparation() + " " + s.getSalt());
                for (long seed : seeds.subList(0, 8)) {
                    for (int rx = -1; rx <= 0; rx++) {
                        for (int rz = -1; rz <= 0; rz++) {
                            CPos p = s.getInRegion(seed, rx, rz, rand);
                            System.out.println("REG " + e.getKey() + " " + v + " " + seed + " " + rx + " " + rz + " " + pos(p));
                            if (p != null) {
                                for (int[] d : new int[][]{{0, 0}, {1, 0}, {0, -1}}) {
                                    int cx = p.getX() + d[0], cz = p.getZ() + d[1];
                                    RegionStructure.Data<?> data = s.at(cx, cz);
                                    boolean a = data.testStart(seed, rand);
                                    boolean b = data.testStart(seed ^ 0x10001L, rand);
                                    System.out.println("START " + e.getKey() + " " + v + " " + seed + " " + cx + " " + cz + " " + a + " " + b);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Decorators of the library that SeedCrackerX uses: desert well and end gateway.
        for (MCVersion v : new MCVersion[]{MCVersion.v1_13, MCVersion.v1_16_5, MCVersion.v1_17, MCVersion.v1_21_3}) {
            DesertWell well = new DesertWell(v);
            EndGateway gateway = new EndGateway(v);
            for (long seed : seeds.subList(0, 8)) {
                for (int cx = -1; cx <= 1; cx++) {
                    for (int cz = -1; cz <= 1; cz++) {
                        DesertWell.Data wd = well.getData(seed, cx, cz, rand);
                        if (wd != null) {
                            System.out.println("WELL " + v + " " + seed + " " + cx + " " + cz + " " + wd.blockX + " " + wd.blockZ + " " + wd.testStart(seed, rand) + " " + well.at(wd.blockX + 1, wd.blockZ).testStart(seed, rand));
                        } else {
                            System.out.println("WELL " + v + " " + seed + " " + cx + " " + cz + " none");
                        }
                        EndGateway.Data gd = gateway.getData(seed, cx, cz, rand);
                        if (gd != null) {
                            System.out.println("GATE " + v + " " + seed + " " + cx + " " + cz + " " + gd.blockX + " " + gd.blockZ + " " + gd.height + " " + gd.testStart(seed, rand) + " " + gateway.at(gd.blockX, gd.blockZ, gd.height + 1).testStart(seed, rand));
                        } else {
                            System.out.println("GATE " + v + " " + seed + " " + cx + " " + cz + " none");
                        }
                    }
                }
            }
        }

        // The seedings of ChunkRand.
        for (MCVersion v : new MCVersion[]{MCVersion.v1_12_2, MCVersion.v1_13, MCVersion.v1_16_5, MCVersion.v1_21_3}) {
            for (long w : worldSeeds) {
                for (int[] c : new int[][]{{0, 0}, {5, -7}, {-100, 33}}) {
                    ChunkRand cr = new ChunkRand();
                    StringBuilder sb = new StringBuilder("SEEDING " + v + " " + w + " " + c[0] + " " + c[1]);
                    sb.append(" ").append(cr.setTerrainSeed(c[0], c[1], v)).append("/").append(cr.nextInt());
                    sb.append(" ").append(cr.setPopulationSeed(w, c[0] << 4, c[1] << 4, v)).append("/").append(cr.nextInt());
                    if (v.isNewerOrEqualTo(MCVersion.v1_13)) {
                        sb.append(" ").append(cr.setDecoratorSeed(w, c[0] << 4, c[1] << 4, 3, 2, v)).append("/").append(cr.nextInt());
                        sb.append(" ").append(cr.setDecoratorSeed(w, c[0] << 4, c[1] << 4, 20003, v)).append("/").append(cr.nextInt());
                    }
                    sb.append(" ").append(cr.setCarverSeed(w, c[0], c[1], v)).append("/").append(cr.nextInt());
                    sb.append(" ").append(cr.setRegionSeed(w, c[0], c[1], 14357617, v)).append("/").append(cr.nextInt());
                    sb.append(" ").append(cr.setWeakSeed(w, c[0], c[1], v)).append("/").append(cr.nextInt());
                    sb.append(" ").append(cr.setSlimeSeed(w, c[0], c[1], v)).append("/").append(cr.nextInt());
                    sb.append(" ").append(cr.setPositionSeed(c[0] * 7, 64, c[1] * 3, v)).append("/").append(cr.nextInt());
                    sb.append(" ").append(cr.setBaseStoneSeed(w, c[0], 5, c[1])).append("/").append(cr.nextInt());
                    System.out.println(sb);
                }
            }
        }

        // Seeds.
        for (long s : seeds) {
            long structure = s & 0xFFFFFFFFFFFFL;
            StringBuilder sb = new StringBuilder("SEED " + s);
            sb.append(" pillar=").append(PillarSeed.fromStructureSeed(structure));
            sb.append(" hash=").append(WorldSeed.toHash(s));
            sb.append(" shadow=").append(WorldSeed.getShadowSeed(s));
            sb.append(" random=").append(WorldSeed.isRandom(s));
            sb.append(" string=").append(WorldSeed.isString(s));
            List<Long> rw = StructureSeed.toRandomWorldSeeds(structure);
            sb.append(" randomWorlds=").append(rw);
            int[] h = PillarSeed.getPillarHeights(structure & 0xFFFF);
            sb.append(" heights=").append(Arrays.toString(h));
            System.out.println(sb);
        }
        for (long w : worldSeeds) {
            long structure = w & 0xFFFFFFFFFFFFL;
            System.out.println("RAND " + w + " " + WorldSeed.isRandom(w) + " " + StructureSeed.toRandomWorldSeeds(structure) + " " + WorldSeed.toHash(w));
        }
        // Worlds made by nextLong.
        for (int i = 0; i < 20; i++) {
            long internal = r.nextLong() & 0xFFFFFFFFFFFFL;
            long w = JRand.ofInternalSeed(internal).nextLong();
            System.out.println("NEXTLONG " + internal + " " + w + " " + StructureSeed.toRandomWorldSeeds(w & 0xFFFFFFFFFFFFL) + " " + WorldSeed.isRandom(w));
        }
        // JRand.
        for (long s : seeds.subList(0, 8)) {
            JRand j = new JRand(s);
            StringBuilder sb = new StringBuilder("JRAND " + s);
            sb.append(" ").append(j.nextInt()).append(" ").append(j.nextInt(24)).append(" ").append(j.nextLong()).append(" ").append(j.nextDouble()).append(" ").append(j.nextFloat()).append(" ").append(j.nextBoolean()).append(" ").append(j.nextInt(16)).append(" ").append(j.nextInt(7)).append(" ").append(j.nextGaussian());
            System.out.println(sb);
        }
    }
}
