package com.example.rustyxr.broker;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Typeface;
import android.os.Bundle;
import android.util.Log;
import android.view.Gravity;
import android.view.Window;
import android.view.WindowManager;
import android.widget.LinearLayout;
import android.widget.TextView;

public final class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        Intent serviceIntent = new Intent(this, BrokerService.class);
        Intent launchIntent = getIntent();
        if (launchIntent != null && launchIntent.getExtras() != null) {
            serviceIntent.putExtras(launchIntent);
        }
        startService(serviceIntent);
        Log.i(BrokerService.TAG, "MainActivity launched broker service");

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setGravity(Gravity.START);
        root.setPadding(32, 28, 32, 28);
        root.setBackgroundColor(Color.rgb(14, 17, 19));

        TextView title = textView(24, true);
        title.setText("Rusty XR Broker");
        root.addView(title);

        TextView status = textView(16, false);
        status.setTypeface(Typeface.MONOSPACE);
        status.setText(
            "Service: started\n" +
            "HTTP status: http://127.0.0.1:8765/status\n" +
            "WebSocket: ws://127.0.0.1:8765/rustyxr/v1/events\n" +
            "Protocol: rusty.xr.broker.latency.v1\n" +
            "LSL: native publisher when bundled liblsl loads\n" +
            "OSC egress: rustyxr.oscEnabled/Host/Port\n" +
            "OSC ingress: rustyxr.oscIngressEnabled/Port\n" +
            "Diagnostics: adb logcat -s RustyXrBroker");
        root.addView(status);

        setContentView(root);
    }

    private TextView textView(int sizeSp, boolean header) {
        TextView view = new TextView(this);
        view.setTextColor(header ? Color.rgb(244, 248, 239) : Color.rgb(208, 218, 214));
        view.setTextSize(sizeSp);
        view.setPadding(0, 0, 0, 14);
        return view;
    }
}
