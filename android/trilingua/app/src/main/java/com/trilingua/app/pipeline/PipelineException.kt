package com.trilingua.app.pipeline

import com.trilingua.app.model.TrilinguaError

/** Thrown by engine impls to surface a known error variant without losing type information. */
class PipelineException(val error: TrilinguaError) : Exception(error::class.simpleName)
