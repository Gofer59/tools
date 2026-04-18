package com.trilingua.app

import android.app.Application
import com.trilingua.app.di.AppContainer
import com.trilingua.app.util.Logger

class TrilinguaApplication : Application() {
    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        Logger.i("TrilinguaApplication: onCreate")
        container = AppContainer(this)
    }
}
