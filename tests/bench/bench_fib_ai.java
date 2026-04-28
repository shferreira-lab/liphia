import java.util.Arrays;

public class BenchFibAI {

    static int fibRec(int n){
        if(n<=1) return n;
        return fibRec(n-1)+fibRec(n-2);
    }

    static int fibIter(int n){
        int a=0,b=1;
        for(int i=0;i<n;i++){
            int temp=a;
            a=b;
            b=temp+b;
        }
        return a;
    }

    static double dot(double[] v1,double[] v2){
        double sum=0; for(int i=0;i<v1.length;i++) sum+=v1[i]*v2[i]; return sum;
    }

    static double norm(double[] v){ double sum=0; for(double x:v) sum+=x*x; return Math.sqrt(sum); }

    static double[] softmax(double[] v){
        double[] exp = new double[v.length];
        double sum=0;
        for(int i=0;i<v.length;i++){ exp[i]=Math.exp(v[i]); sum+=exp[i]; }
        for(int i=0;i<v.length;i++) exp[i]/=sum;
        return exp;
    }

    public static void main(String[] args){
        double[] v1={1,2,3,4,5};
        double[] v2={5,4,3,2,1};

        long start = System.nanoTime();
        int r1 = fibRec(25);
        long t1 = System.nanoTime()-start;

        start = System.nanoTime();
        int r2 = fibIter(35);
        long t2 = System.nanoTime()-start;

        start = System.nanoTime();
        double dot_val = dot(v1,v2);
        double norm_val = norm(v1);
        double[] sm = softmax(v1);
        long t3 = System.nanoTime()-start;

        System.out.println("fibRec(25) = "+r1+" | tempo = "+t1/1e6+" ms");
        System.out.println("fibIter(35) = "+r2+" | tempo = "+t2/1e6+" ms");
        System.out.println("dot(v1,v2) = "+dot_val);
        System.out.println("norm(v1) = "+norm_val);
        System.out.println("softmax(v1) = "+Arrays.toString(sm)+" | tempo = "+t3/1e6+" ms");
    }
}