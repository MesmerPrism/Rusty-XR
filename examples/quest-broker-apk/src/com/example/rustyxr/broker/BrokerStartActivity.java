package com.example.rustyxr.broker;

import android.app.Activity;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import android.util.Log;

public final class BrokerStartActivity extends Activity {
    @Override
    protected void onCreate(Bundle bundle) {
        super.onCreate(bundle);

        Intent serviceIntent = new Intent(this, BrokerService.class);
        Intent launchIntent = getIntent();
        if (launchIntent != null && launchIntent.getExtras() != null) {
            serviceIntent.putExtras(launchIntent);
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(serviceIntent);
        } else {
            startService(serviceIntent);
        }

        Log.i(BrokerService.TAG, "BrokerStartActivity launched broker service");
        finish();
        overridePendingTransition(0, 0);
    }
}
