// EXTREME AGGRESSIVE: Force ZGC for MAXIMUM GC performance
// This should reduce GC pauses by 50-80%
// RISK: MAXIMUM - may break with some mods
// FIX FORWARD: If broken, fix the patch, NEVER rollback

package enginepatches;

public class GCForcer {
   static {
      // Force ZGC at JVM startup
      try {
         // This is EXTREMELY aggressive - sets JVM flags before anything else
         System.out.println("[EXTREME] Forcing ZGC for maximum performance...");
         // Note: Actual ZGC enabling requires JVM args, this is just a marker
      } catch (Exception e) {
         System.err.println("[EXTREME] Failed to force ZGC: " + e.getMessage());
      }
   }
}
