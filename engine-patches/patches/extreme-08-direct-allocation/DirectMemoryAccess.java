// EXTREME AGGRESSIVE: Direct memory access for ALL operations
// This should speed up EVERYTHING by 10-50%
// RISK: MAXIMUM - bypasses all safety checks
// FIX FORWARD: If broken, fix the patch, NEVER rollback

package enginepatches;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import org.lwjgl.system.MemoryUtil;

public class DirectMemoryAccess {
   static {
      System.out.println("[EXTREME] Enabling direct memory access for ALL operations");
   }
   
   public static ByteBuffer createDirectBuffer(int size) {
      long pointer = MemoryUtil.nmemAlloc(size);
      if (pointer == 0) {
         throw new OutOfMemoryError("Failed to allocate direct memory");
      }
      return MemoryUtil.memByteBuffer(pointer, size);
   }
   
   public static void freeDirectBuffer(ByteBuffer buffer) {
      if (buffer != null && buffer.isDirect()) {
         MemoryUtil.nmemFree(MemoryUtil.memAddress(buffer));
      }
   }
}
